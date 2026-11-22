//! The PES 2014 save: every screen, menu and the roster drag works, with
//! each control whose field the version does not store greyed.

use pes_savefile::interchange::team_toml::TeamToml;

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

/// The committed PES 2014 fixture with team 1 renumbered 701 and an
/// appearance record per player its roster fills (`missing` left out),
/// written into the section the layout reserves between the player and
/// team sections: the version holds a record for an edited or created
/// player alone, so the injected records land where `from_bytes` reads
/// them, over whatever the game's save left in the space.
fn pes14_fixture_with_look(test: &str) -> PathBuf {
    pes14_fixture_with_look_missing(test, &[])
}

/// The committed PES 2014 fixture as it is, team 1 renumbered 701, the
/// cup id the tests' selections name.
fn pes14_fixture_plain(test: &str) -> PathBuf {
    let path = fixture(PesVersion::Pes14, test);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
    file.team_mut(1).expect("the vanilla first team").id = 701;
    file.save(&path).unwrap();
    path
}

/// `pes14_fixture_with_look` leaving `missing`'s records out: those
/// players keep no record, as an unedited PES 2014 player's.
fn pes14_fixture_with_look_missing(test: &str, missing: &[u32]) -> PathBuf {
    use pes_savefile::container::SaveContainer;
    use pes_savefile::schema::schema_for;

    let ids: Vec<u32> = EditFile::from_bytes(&fixture_save(PesVersion::Pes14))
        .unwrap()
        .team(1)
        .expect("the vanilla first team")
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0 && !missing.contains(&slot.player_id))
        .map(|slot| slot.player_id)
        .collect();
    let mut container =
        SaveContainer::decrypt(&fixture_save(PesVersion::Pes14)).expect("the fixture decrypts");
    let (layout, appearance) = schema_for(PesVersion::Pes14)
        .appearance
        .as_ref()
        .expect("PES 2014 has an appearance section");
    for (i, id) in ids.iter().enumerate() {
        let at = layout.offset + i * appearance.size;
        container.payload[at..at + 4].copy_from_slice(&id.to_le_bytes());
    }
    container.payload[layout.count_offset..layout.count_offset + 2].copy_from_slice(
        &u16::try_from(ids.len())
            .expect("a roster fits u16")
            .to_le_bytes(),
    );
    let mut file =
        EditFile::from_bytes(&container.to_bytes(&crate::test_support::TEST_SALT).unwrap())
            .unwrap();
    file.team_mut(1).expect("the vanilla first team").id = 701;
    let dir = crate::test_support::scratch(test);
    let path = dir.join("EDIT00000000");
    file.save(&path).unwrap();
    path
}

// A PES 2014 save lists every team it holds, not the cup's ids alone —
// the 2014 cup ran at real clubs' ids, and the vanilla fixture holds no
// cup id: All teams' list reaches team 136, VASCO DA GAMA, and the
// filter's search by its name finds it.
#[test]
fn a_pes14_save_lists_every_team_and_the_filter_finds_it() {
    let path = fixture(PesVersion::Pes14, "pes14_all_teams");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // All teams, no filter (the open's own state): team 136's row draws
    // with player rows under it once scrolled to — the list lays out only
    // the rows in view, the reveal a selection's own.
    view.selected = Some(Selection::Team(136));
    view.reveal_selected = Some(Reveal::Top);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let top = team_row(&nodes, &view, 136).bounds().unwrap().y0;
    assert!(
        nodes.iter().filter(|node| player_row(node)).any(|node| node
            .bounds()
            .expect("a row has bounds")
            .y0
            > top),
        "a player row under team 136's row"
    );

    // The filter's search by the team's name finds it: "vasco" typed in
    // the box leaves its row.
    let filter_box = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && node.placeholder() == Some("Team or player: name, shirt name or ID")
        })
        .expect("the filter box");
    click(&mut window, filter_box, &mut view);
    let (_, nodes, _) = window.frame(vec![egui::Event::Text("vasco".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let _ = team_row(&nodes, &view, 136);
}

// Every screen, the four menus and a roster drag on the committed PES 2014
// fixture (a real save: team 1's 23-man squad, given an appearance record
// each by `pes14_fixture_with_look`). Nothing panics; the items the version
// rules out — the in-game face run's and the not-in-scope list's — are
// disabled with the PES 2014 hover, and a drag undone leaves the loaded
// order.
#[test]
fn a_pes14_save_draws_screens_and_menus_and_drags_without_panics() {
    // The tests' selections name a cup id: team 1 takes 701, its roster
    // along.
    let path = pes14_fixture_with_look("pes14_screens");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // A rostered player selected: the Stats screen draws, then the Player
    // menu's items gate (the two Generate items need the in-game face run
    // the version's records carry none of; the FPC entry reads the ids it
    // needs).
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
    // The FPC entry is live — the version stores the ids it writes, and
    // the selected player holds an appearance record — while the two
    // Generate items grey on the missing in-game face run.
    assert!(
        !menu_item(&nodes, "Hide default body (Enable FPC)").is_disabled(),
        "the FPC entry is enabled on PES 2014"
    );
    for label in [
        "Generate settings.toml\u{2026}",
        "Generate in-game face\u{2026}",
    ] {
        assert_greyed_item(&nodes, &mut window, &mut view, label);
    }

    // The Team menu: the boots and gloves item, the two default-body
    // items and the team file panel's entry stay live (the version
    // stores the ids they write, and Team TOML takes a PES 2014 team
    // whole), the out-of-scope formation picture greyed.
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    for label in [
        "Set boots and gloves IDs for everyone\u{2026}",
        "Hide default body (Enable FPC)",
        "Show default body (Disable FPC)",
        "Import / export team file\u{2026}",
    ] {
        assert!(
            !menu_item(&nodes, label).is_disabled(),
            "the {label} item is enabled on PES 2014"
        );
    }
    assert_greyed_item(
        &nodes,
        &mut window,
        &mut view,
        "Save a picture of the formations\u{2026}",
    );

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

    // The Appearance screen draws: the look controls the appearance
    // record stores edit — the FPC buttons and the gloves fields among
    // them — while the colour cells take the PES 2014 hover (their fields
    // live in the in-game face run alone), and the unstored strip
    // colours, motions and edit flags keep the gated stand-in.
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for label in ["Hide default body", "Show default body"] {
        assert!(
            !node_labeled(&nodes, Role::Button, label).is_disabled(),
            "the {label} button is enabled on PES 2014"
        );
    }
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Boots ID")
    });
    for label in ["Boots ID", "Gloves ID", "Sleeves", "Outfielder gloves"] {
        assert!(
            !control_right_of(&nodes, label).is_disabled(),
            "the {label} control is enabled on PES 2014"
        );
    }
    for label in ["Spectacles color", "Gloves color"] {
        disabled_dash(&nodes, label);
    }
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Hunching (dribbling)")
    });
    disabled_dash(&nodes, "Hunching (dribbling)");
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
    assert!(
        !control_right_of(&nodes, "Neck length").is_disabled(),
        "the physique sliders edit on PES 2014"
    );
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::CheckBox && node.label() == Some("Face")
    });
    assert!(
        !node_labeled(&nodes, Role::CheckBox, "Face").is_disabled(),
        "the face flag's box is enabled on PES 2014"
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

// The Gameplan screen on PES 2014: the toggle's segment is enabled, a
// team row draws the tactics editor's PES 2014 frame (the preset bar,
// the Scroll-only formation bar and the Preset tactics panel, with no
// Fluid formation, Portraits, formation selector or Advanced
// instructions), the Edit menu's Copy and Paste take a Gameplan target
// — they carry Team TOML, which a PES 2014 save takes — and the team
// file panel's controls work: a file staged for it enables the Gameplan
// section and Apply, which writes the file's Game Plan.
#[test]
fn a_pes14_save_draws_the_gameplan_and_applies_a_team_file() {
    let path = pes14_fixture_plain("pes14_gameplan");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // A rostered player is selected on open, so the toggle draws: its
    // Gameplan segment is enabled.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !node_labeled(&nodes, Role::Button, "Gameplan").is_disabled(),
        "the Gameplan segment is enabled on PES 2014"
    );

    // A team row draws the header's fields and the tactics editor's PES
    // 2014 frame: the Preset tactics panel where Formation settings
    // would stand, the Scroll switch alone on the formation bar — no
    // Fluid formation, Portraits, formation selector or Advanced
    // instructions — and the preset bar's Copy/swap menu alone.
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
    assert!(
        label_value(&nodes, "Preset tactics"),
        "the Preset tactics panel draws"
    );
    assert!(
        label_value(&nodes, "Player assignments") && label_value(&nodes, "Automatic settings"),
        "the assignments and automatic settings panels draw"
    );
    for absent in ["Formation settings", "Advanced instructions"] {
        assert!(
            !label_value(&nodes, absent),
            "no {absent} panel on PES 2014"
        );
    }
    for absent in ["Fluid formation", "Portraits"] {
        assert!(
            !nodes
                .iter()
                .any(|node| node.role() == Role::CheckBox && node.label() == Some(absent)),
            "no {absent} switch on PES 2014"
        );
    }
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Role::CheckBox && node.label() == Some("Scroll")),
        "the formation bar's Scroll switch draws"
    );
    for absent in ["Kick-off", "In-possession", "Out-of-possession"] {
        assert!(
            !nodes
                .iter()
                .any(|node| node.role() == Role::Button && node.label() == Some(absent)),
            "no formation selector on PES 2014"
        );
    }
    let copy_swaps = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button && node.label() == Some(menu_label("Copy/swap").as_str())
        })
        .count();
    assert_eq!(copy_swaps, 1, "the preset bar's Copy/swap menu alone");

    // The Edit menu's Copy and Paste are live on the Gameplan target a
    // team row is.
    let nodes = open_menu(&mut window, &mut view, "Edit", "Copy");
    for label in ["Copy", "Paste"] {
        assert!(
            !menu_item(&nodes, label).is_disabled(),
            "{label} is enabled on a PES 2014 Gameplan"
        );
    }

    // The team file panel, opened as the menu's item opens it (the menu
    // check leaves the Edit menu open, closed here): the export and pick
    // buttons are enabled on PES 2014.
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
            !node_labeled(&panel, Role::Button, label).is_disabled(),
            "the {label} button is enabled on PES 2014"
        );
    }

    // A PES 2014 file of the team with preset 1's Strategy changed,
    // staged for it: the Gameplan section and Apply are enabled, and
    // Apply writes the file's Game Plan.
    let mut document = {
        let file = view.session.as_ref().expect("an open save").file();
        TeamToml::from_team(PesVersion::Pes14, file.team(701).expect("team 701"), &[])
            .expect("the export")
    };
    let plan = document.tactics.presets[0]
        .game_plan
        .as_mut()
        .expect("a PES 2014 preset holds its Game Plan");
    assert_eq!(plan.strategy, Some(1), "premise: preset 1's Strategy");
    plan.strategy = Some(5);
    let toml = path.with_file_name("picked.pes14.toml");
    std::fs::write(&toml, document.to_toml().expect("the document")).unwrap();
    let recorder = view.session.as_ref().expect("an open save").recorder();
    view.interchange.pick(toml, recorder);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let panel: Vec<accesskit::Node> = nodes_in_window(&pairs, "Team file interchange")
        .into_iter()
        .map(|(_, node)| node.clone())
        .collect();
    assert!(
        !node_labeled(&panel, Role::CheckBox, "Gameplan").is_disabled(),
        "the Gameplan section is enabled on PES 2014"
    );
    let apply = node_labeled(&panel, Role::Button, "Apply");
    assert!(!apply.is_disabled(), "Apply is enabled on PES 2014");
    click(&mut window, apply, &mut view);

    assert_eq!(
        team_entry(&view, 701).tactics.presets[0]
            .game_plan
            .expect("a game plan")
            .strategy,
        5,
        "Apply wrote the file's Strategy"
    );
    view.panel = None;
}

/// The bounds of every Label reading `label`, top to bottom.
fn label_bounds_all(
    ids: &[(accesskit::NodeId, accesskit::Node)],
    label: &str,
) -> Vec<accesskit::Rect> {
    let mut found: Vec<accesskit::Rect> = ids
        .iter()
        .filter(|(_, node)| node.role() == Role::Label && node.value() == Some(label))
        .filter_map(|(_, node)| node.bounds())
        .collect();
    found.sort_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"));
    found
}

/// The AccessKit ids of the `role` widgets under `header`, in its column,
/// top to bottom.
fn ids_below(
    ids: &[(accesskit::NodeId, accesskit::Node)],
    header: accesskit::Rect,
    role: Role,
) -> Vec<accesskit::NodeId> {
    let mut found: Vec<(f64, accesskit::NodeId)> = ids
        .iter()
        .filter(|(_, node)| node.role() == role)
        .filter_map(|(id, node)| {
            let b = node.bounds()?;
            (b.y0 > header.y0 && (b.x0 - header.x0).abs() < 1.0).then_some((b.y0, *id))
        })
        .collect();
    found.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered bounds"));
    found.into_iter().map(|(_, id)| id).collect()
}

/// The AccessKit id of the `role` widget under the topmost Label reading
/// `label`, in that label's column (a Preset tactics cell's control:
/// Combination play's "Strategy" header stands below Preset tactics').
fn control_id_below(
    ids: &[(accesskit::NodeId, accesskit::Node)],
    label: &str,
    role: Role,
) -> accesskit::NodeId {
    let bounds = *label_bounds_all(ids, label)
        .first()
        .unwrap_or_else(|| panic!("no label {label:?}"));
    *ids_below(ids, bounds, role)
        .first()
        .unwrap_or_else(|| panic!("no {role:?} below {label:?}"))
}

/// The Combination play panel's three rows, top to bottom: each slot's
/// strategy combo, area combo and Clear button. The panel's "Strategy"
/// header is the one level with its "Area" header, since Preset tactics
/// has a "Strategy" label of its own.
fn combination_rows(ids: &[(accesskit::NodeId, accesskit::Node)]) -> [[accesskit::NodeId; 3]; 3] {
    let area = *label_bounds_all(ids, "Area")
        .first()
        .expect("the Area header");
    let strategy = label_bounds_all(ids, "Strategy")
        .into_iter()
        .find(|bounds| (bounds.y0 - area.y0).abs() < 1.0)
        .expect("the panel's Strategy header, level with Area");
    let strategies = ids_below(ids, strategy, Role::ComboBox);
    let areas = ids_below(ids, area, Role::ComboBox);
    let mut clears: Vec<(f64, accesskit::NodeId)> = ids
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some("Clear"))
        .map(|(id, node)| (node.bounds().expect("bounds").y0, *id))
        .collect();
    clears.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered bounds"));
    assert_eq!(
        (strategies.len(), areas.len(), clears.len()),
        (3, 3, 3),
        "three strategy combos, area combos and Clear buttons"
    );
    [0, 1, 2].map(|row| [strategies[row], areas[row], clears[row].1])
}

/// Team 701's Combination Play slots in the view's session, each
/// `(area, strategy)`.
fn combination_of(view: &SaveView) -> [(u8, u8); 3] {
    team_entry(view, 701)
        .tactics
        .combination
        .expect("a PES 2014 record stores Combination Play")
        .map(|slot| (slot.area, slot.strategy))
}

/// Opens combo `combo` and clicks its entry `item`.
fn pick_entry(window: &mut Headless, view: &mut SaveView, combo: accesskit::NodeId, item: &str) {
    action_click(window, view, combo);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    action_click(window, view, item_id(&ids, item));
}

/// The node of AccessKit `id`, among `ids`.
fn node_of(
    ids: &[(accesskit::NodeId, accesskit::Node)],
    id: accesskit::NodeId,
) -> &accesskit::Node {
    ids.iter()
        .find(|(node_id, _)| *node_id == id)
        .map(|(_, node)| node)
        .expect("the node of the id")
}

/// `item`'s AccessKit id among `ids`: the Button of that label.
fn item_id(ids: &[(accesskit::NodeId, accesskit::Node)], item: &str) -> accesskit::NodeId {
    ids.iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(item))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no item {item:?}"))
}

// The Preset tactics panel on the PES 2014 fixture's team 1, IRELAND
// (the record takes 701's id, the cup id the tests' selections name):
// the Formation combo lists the game's eleven, a formation it does not
// hold marked "(not in the game's list)" and offered first; a "4-3-3"
// pick writes the list's slots and keeper depth 3 into the shown preset
// alone, one undo entry; a "Quick Counter" pick writes its index and
// sliders, IRELAND's own Position Switch kept; a slider notch keeps the
// strategy.
#[test]
fn the_preset_tactics_panel_edits_irelands_game_plan() {
    use team_widgets::tactics::pes14;

    let path = pes14_fixture_plain("pes14_preset_tactics");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_team_view(&mut window, &mut view, 701);

    // The premises `pes14_tactics_literals` pins: presets 1 to 3 hold
    // strategies 1, 3 and 4, the shown sliders 13, 8, 9, 9, 8, 12, 10,
    // 10, and the same formation, which the game's list does not hold.
    let before = team_entry(&view, 701).tactics.clone();
    for (index, strategy) in [(0, 1u8), (1, 3), (2, 4)] {
        let preset = &before.presets[index];
        let plan = preset.game_plan.expect("a game plan");
        assert_eq!(plan.strategy, strategy, "preset {}'s strategy", index + 1);
        assert_eq!(
            plan.sliders,
            [13, 8, 9, 9, 8, 12, 10, 10],
            "preset {}'s sliders",
            index + 1
        );
        assert_eq!(
            preset.formations[0],
            before.presets[0].formations[0],
            "preset {}'s formation is preset 1's",
            index + 1
        );
    }
    assert_eq!(
        pes14::listed_formation(&before.presets[0].formations[0]),
        None,
        "premise: IRELAND's formation is not in the game's list"
    );

    // The Formation combo shows the shape with the "(not in the game's
    // list)" mark; its popup offers the mark first, then the eleven.
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let formation_combo = control_id_below(&ids, "Formation", Role::ComboBox);
    assert_eq!(
        node_of(&ids, formation_combo).value(),
        Some("4-2-2-2 (not in the game's list)"),
        "the unlisted formation's mark"
    );
    action_click(&mut window, &mut view, formation_combo);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let mut offered: Vec<&accesskit::Node> = ids
        .iter()
        .map(|(_, node)| node)
        .filter(|node| {
            node.role() == Role::Button
                && node.label().is_some_and(|label| {
                    label == "4-2-2-2 (not in the game's list)"
                        || pes14::LIST_FORMATIONS
                            .iter()
                            .any(|(name, _)| *name == label)
                })
        })
        .collect();
    offered.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().expect("bounds").y0;
        y(a).partial_cmp(&y(b)).expect("ordered bounds")
    });
    assert_eq!(
        offered.first().and_then(|node| node.label()),
        Some("4-2-2-2 (not in the game's list)"),
        "the unlisted formation is offered first"
    );
    action_click(&mut window, &mut view, item_id(&ids, "4-3-3"));

    // The pick writes the list's slots and keeper depth 3 into the
    // shown preset's formation 0 alone — one undo entry.
    let picked = team_entry(&view, 701).tactics;
    let formation = &picked.presets[0].formations[0];
    assert_eq!(
        pes14::listed_formation(formation),
        Some(6),
        "4-3-3 is the list's seventh entry"
    );
    assert_eq!(
        formation.players[10],
        FormationSlot {
            position: 12,
            x: 52,
            y: 43
        },
        "4-3-3's centre-forward slot"
    );
    assert_eq!(
        formation.players[0],
        FormationSlot {
            position: 0,
            x: 52,
            y: 3
        },
        "the keeper keeps his width and position, depth 3"
    );
    for index in 1..3 {
        assert_eq!(
            picked.presets[index],
            before.presets[index],
            "preset {} untouched",
            index + 1
        );
    }

    // The combo now shows "4-3-3"; the pick was one undo entry (the
    // undo restores IRELAND's own formation for the picks below).
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        node_of(&ids, formation_combo).value(),
        Some("4-3-3"),
        "the combo shows the pick"
    );
    assert_eq!(
        undo_entries(&mut window, &mut view),
        1,
        "the formation pick was one undo entry"
    );

    // "Quick Counter" writes strategy 2 and its sliders, IRELAND's own
    // Position Switch (9) kept.
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let strategy_combo = control_id_below(&ids, "Strategy", Role::ComboBox);
    action_click(&mut window, &mut view, strategy_combo);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    action_click(&mut window, &mut view, item_id(&ids, "Quick Counter"));
    let plan = team_entry(&view, 701).tactics.presets[0]
        .game_plan
        .expect("a game plan");
    assert_eq!(plan.strategy, 2, "Quick Counter is the third entry");
    assert_eq!(
        plan.sliders,
        [10, 10, 9, 15, 15, 15, 15, 10],
        "its sliders, Position Switch kept"
    );

    // A notch over the Player support slider moves it alone: the
    // strategy stays.
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let slider = control_id_below(&ids, "Player support", Role::Slider);
    let at = centre(node_of(&ids, slider));
    notch(&mut window, &mut view, at, false);
    let plan = team_entry(&view, 701).tactics.presets[0]
        .game_plan
        .expect("a game plan");
    assert_eq!(plan.strategy, 2, "the slider move keeps the strategy");
    assert_eq!(plan.sliders[0], 11, "a notch down over the rail adds one");
    assert_eq!(
        plan.sliders[1..],
        [10, 9, 15, 15, 15, 15, 10],
        "the other sliders are untouched"
    );
}

// The Formation combo's "(custom)" mark on the PES 2014 fixture's team
// 1 (taking 701's id): preset 1 holds the list's 4-4-2 with one
// outfielder moved sideways, so no list entry matches it while the
// shape still names one. The game itself names a moved 4-4-2 "4-4-2",
// so "(not in the game's list)" would be wrong.
#[test]
fn the_formation_combo_marks_a_moved_list_shape_custom() {
    use team_widgets::tactics::pes14;

    let path = fixture(PesVersion::Pes14, "pes14_custom_formation");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
    let team = file.team_mut(1).expect("the vanilla first team");
    team.id = 701;
    let formation = &mut team.tactics.presets[0].formations[0];
    pes14::pick_formation(formation, 0);
    formation.players[1].x += 2;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_team_view(&mut window, &mut view, 701);

    let moved = &team_entry(&view, 701).tactics.presets[0].formations[0];
    assert_eq!(
        pes14::listed_formation(moved),
        None,
        "premise: the moved 4-4-2 is no list entry"
    );

    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let formation_combo = control_id_below(&ids, "Formation", Role::ComboBox);
    assert_eq!(
        node_of(&ids, formation_combo).value(),
        Some("4-4-2 (custom)"),
        "a moved listed shape is the team's own version of the entry"
    );
}

// The Combination play panel on the PES 2014 fixture's team 1, IRELAND
// (taking 701's id), whose slots hold Dummy Run and Cross in area 2, Hold
// up the ball in 6 and Hold Up and Overlap in 8. The Area header's
// tooltip gives the game's numbering. Cut Inside (areas 3 and 5) picked
// in slot 1 lands on 3, where the game's picker opens; in slot 2 it
// lands on 5, 3 being held. Slot 3 then finds it greyed, and a click on
// it picks nothing. An area pick sets the area alone, Clear empties its
// slot, and the automatic switch sits beside Offside trap. The save holds
// each as the game's bytes.
#[test]
fn the_combination_play_panel_edits_irelands_slots() {
    let path = pes14_fixture_plain("pes14_combination_play");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_team_view(&mut window, &mut view, 701);
    assert_eq!(
        combination_of(&view),
        [(2, 10), (6, 1), (8, 5)],
        "premise: IRELAND's slots, as `pes14_tactics_literals` pins them"
    );
    assert_eq!(
        team_entry(&view, 701).tactics.auto.combination_play,
        Some(false),
        "premise: the automatic switch is off"
    );

    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let area_header = *label_bounds_all(&ids, "Area")
        .first()
        .expect("the Area header");
    let at = egui::pos2(
        ((area_header.x0 + area_header.x1) / 2.0) as f32,
        ((area_header.y0 + area_header.y1) / 2.0) as f32,
    );
    assert_eq!(
        tooltip_containing(&mut window, &mut view, at, "area screen"),
        Some(
            "The game's area screen numbers the pitch in rows, from its top: 1 and 2, then 3 to \
             5, 6 to 8, and 9 to 11."
                .to_string()
        ),
        "the Area header's tooltip"
    );

    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows = combination_rows(&ids);
    assert_eq!(
        node_of(&ids, rows[0][0]).value(),
        Some("Dummy Run and Cross"),
        "slot 1's strategy by name"
    );
    pick_entry(&mut window, &mut view, rows[0][0], "Cut Inside");
    assert_eq!(
        combination_of(&view),
        [(3, 3), (6, 1), (8, 5)],
        "Cut Inside in slot 1 opens on 3"
    );
    pick_entry(&mut window, &mut view, rows[1][0], "Cut Inside");
    assert_eq!(
        combination_of(&view),
        [(3, 3), (5, 3), (8, 5)],
        "Cut Inside in slot 2 lands on 5, 3 being held"
    );

    // Slot 3's list greys Cut Inside, and a click on it picks nothing.
    action_click(&mut window, &mut view, rows[2][0]);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let cut_inside = item_id(&ids, "Cut Inside");
    assert!(
        node_of(&ids, cut_inside).is_disabled(),
        "Cut Inside is greyed for slot 3"
    );
    assert!(
        !node_of(&ids, item_id(&ids, "Hold up the ball")).is_disabled(),
        "a strategy with a free area stays live"
    );
    action_click(&mut window, &mut view, cut_inside);
    assert_eq!(
        combination_of(&view),
        [(3, 3), (5, 3), (8, 5)],
        "the greyed entry picks nothing"
    );
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );

    // Slot 3's areas are the midfield's less 3 and 5: an area pick sets
    // the area alone.
    // The popup's entries are the buttons its opening added (the preset
    // bar's are numbers too).
    let (_, closed, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    action_click(&mut window, &mut view, combination_rows(&closed)[2][1]);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let mut entries: Vec<(f64, &str, accesskit::NodeId)> = ids
        .iter()
        .filter(|(id, node)| {
            node.role() == Role::Button && !closed.iter().any(|(shown, _)| shown == id)
        })
        .filter_map(|(id, node)| Some((node.bounds()?.y0, node.label()?, *id)))
        .collect();
    entries.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered bounds"));
    assert_eq!(
        entries
            .iter()
            .map(|(_, label, _)| *label)
            .collect::<Vec<_>>(),
        ["4", "6", "7", "8"],
        "slot 3's free areas"
    );
    action_click(&mut window, &mut view, entries[0].2);
    assert_eq!(
        combination_of(&view),
        [(3, 3), (5, 3), (4, 5)],
        "the area pick keeps the strategy"
    );

    // Clear empties slot 2: "None Selected", its area combo and Clear
    // disabled.
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    action_click(&mut window, &mut view, combination_rows(&ids)[1][2]);
    assert_eq!(
        combination_of(&view),
        [(3, 3), (0, 0), (4, 5)],
        "Clear stores strategy 0 and area 0"
    );
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows = combination_rows(&ids);
    assert_eq!(
        node_of(&ids, rows[1][0]).value(),
        Some("None Selected"),
        "an empty slot reads None Selected"
    );
    assert!(
        node_of(&ids, rows[1][1]).is_disabled() && node_of(&ids, rows[1][2]).is_disabled(),
        "an empty slot's area combo and Clear are disabled"
    );

    // The automatic switch.
    let switch = ids
        .iter()
        .find(|(_, node)| node.role() == Role::CheckBox && node.label() == Some("Combination play"))
        .map(|(id, _)| *id)
        .expect("Automatic settings' Combination play switch");
    action_click(&mut window, &mut view, switch);
    assert_eq!(
        team_entry(&view, 701).tactics.auto.combination_play,
        Some(true),
        "the switch is on"
    );

    // The save holds the switch at +0x1D3 and the slots, area then
    // strategy, from +0x1D6.
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let saved = EditFile::load(&path).unwrap();
    let record = saved.team_records(701).expect("team 701").tactics;
    assert_eq!(record[0x1D3], 1, "Combination Play's switch");
    assert_eq!(
        record[0x1D6..0x1DC],
        [3, 3, 0, 0, 4, 5],
        "the slots' area and strategy bytes"
    );
}

/// PES 2014's own `all.str` (`pes_strings`' fixture).
const ALL_STR_FILE: &[u8] =
    include_bytes!("../../../../../libs/pes_strings/tests/fixtures/pes14_all.str");

/// A PES folder holding the fixture's `all.str` as `Data/dt21_eng.cpk`'s
/// entry, under `path`'s scratch (the fixture save sits at `path`).
fn pes_folder_with_all_str(path: &std::path::Path) -> std::path::PathBuf {
    let folder = path.parent().expect("a file under scratch").join("PES");
    crate::test_support::write_cpk(
        &folder.join("Data/dt21_eng.cpk"),
        &[("pes14_win_dat/eng/string/all.str", ALL_STR_FILE)],
    );
    folder
}

/// `view`'s Gameplan tooltips' read polled until it lands or fails (bounded:
/// a read that never settles fails rather than hangs; a hang guard, not a
/// speed bound, `workers_replied`' 120 s).
fn tips_settled(view: &mut SaveView) {
    let deadline = Instant::now() + Duration::from_secs(120);
    while !view.game_tips.settled() {
        assert!(
            Instant::now() < deadline,
            "the tooltips' read never settled in 120 s"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The centre of the Label reading `label` (a settings cell's label).
fn label_centre(nodes: &[accesskit::Node], label: &str) -> egui::Pos2 {
    centre(
        nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(label))
            .unwrap_or_else(|| panic!("no label {label:?}")),
    )
}

// A PES 2014 folder holding the game's own `all.str`: hovering a slider's
// label shows the setting's help text, word for word from the install, and
// hovering the Combination play title shows its text, the button code
// written "LT".
#[test]
fn the_gameplan_s_tooltips_are_the_install_s_help_texts() {
    let path = pes14_fixture_plain("pes14_tips");
    let folder = pes_folder_with_all_str(&path);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    view.select_portraits(&window.ctx, &folder, PesVersion::Pes14);
    tips_settled(&mut view);
    assert!(
        view.game_tips.tips().is_some(),
        "premise: the archive's texts resolved"
    );
    open_team_view(&mut window, &mut view, 701);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            label_centre(&nodes, "Player support"),
            "ball retention"
        ),
        Some(
            "The higher the value, the more the players lay emphasis on ball retention when positioning themselves. On a lower level, they will look to maintaining the team's overall shape. "
                .to_string()
        ),
        "the slider label's tooltip is the game's own text"
    );

    let title = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("Combination play"))
        .expect("the Combination play title");
    let tip = tooltip_containing(&mut window, &mut view, centre(title), "press LT")
        .expect("the title's tooltip");
    assert!(
        tip.ends_with("press LT twice when the Combination Play icon is shown on screen."),
        "the button code written LT: {tip:?}"
    );
    assert!(!tip.contains('\u{11}'), "no inline code shown: {tip:?}");

    // Every other placement shows its own `Tips` field, end to end: a
    // label's hover equals the field it was bound to, so a binding
    // swapped at its call site fails here.
    let expected = view.game_tips.tips().expect("resolved").set_pieces[0].clone();
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            label_centre(&nodes, "Long free kick"),
            "Long distance"
        ),
        expected,
        "the Long free kick label's tooltip"
    );
    let expected = view.game_tips.tips().expect("resolved").join_attack.clone();
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            label_centre(&nodes, "Join the attack"),
            "Set Pieces"
        ),
        expected,
        "the Join the attack label's tooltip"
    );

    // The Strategy combo's hover is the stored strategy's text (preset 1
    // holds Long Ball on this fixture).
    let strategy = usize::from(
        team_entry(&view, 701).tactics.presets[0]
            .game_plan
            .expect("a game plan")
            .strategy,
    );
    let name = team_widgets::tactics::pes14::STRATEGIES[strategy];
    let combo = nodes
        .iter()
        .find(|node| node.role() == Role::ComboBox && node.value() == Some(name))
        .expect("the Strategy combo");
    let expected = view.game_tips.tips().expect("resolved").strategies[strategy].clone();
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(combo), "tight backline"),
        expected,
        "the Strategy combo's tooltip is the stored strategy's"
    );

    // The Automatic settings' checkbox and label, each its block of the
    // shared text.
    let expected = view
        .game_tips
        .tips()
        .expect("resolved")
        .offside_trap
        .clone();
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            centre(node_labeled(&nodes, Role::CheckBox, "Offside trap")),
            "offside traps during"
        ),
        expected,
        "the Offside trap checkbox's tooltip"
    );
    let expected = view
        .game_tips
        .tips()
        .expect("resolved")
        .substitution
        .clone();
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            label_centre(&nodes, "Substitution"),
            "automatically substituted"
        ),
        expected,
        "the Substitution label's tooltip"
    );
}

// No PES folder set: the Gameplan draws with no game tooltip (nothing was
// read) and does not panic.
#[test]
fn without_a_pes_folder_the_labels_carry_no_game_tooltip() {
    let path = pes14_fixture_plain("pes14_no_folder");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_team_view(&mut window, &mut view, 701);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            label_centre(&nodes, "Player support"),
            "ball retention"
        ),
        None,
        "no game tooltip without a PES folder"
    );
}

// A PES folder that holds no English archive: the read fails quietly, the
// Gameplan draws with no game tooltip and does not panic.
#[test]
fn a_folder_without_the_english_archive_leaves_no_tooltip() {
    let path = pes14_fixture_plain("pes14_no_archive");
    let folder = path.parent().expect("a file under scratch").join("PES");
    std::fs::create_dir_all(&folder).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    view.select_portraits(&window.ctx, &folder, PesVersion::Pes14);
    tips_settled(&mut view);
    assert!(
        view.game_tips.tips().is_none(),
        "premise: no archive, no texts"
    );
    open_team_view(&mut window, &mut view, 701);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            label_centre(&nodes, "Player support"),
            "ball retention"
        ),
        None,
        "no game tooltip without the archive"
    );
}

// A PES 2016 save's Gameplan is unchanged: the folder's archive is never
// read for it and a slider label's hover shows no game tooltip.
#[test]
fn a_pes16_save_s_gameplan_shows_no_game_tooltip() {
    let path = fixture(PesVersion::Pes16, "pes16_no_tips");
    let folder = pes_folder_with_all_str(&path);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    view.select_portraits(&window.ctx, &folder, PesVersion::Pes16);
    tips_settled(&mut view);
    assert!(
        view.game_tips.tips().is_none(),
        "a PES 2016 save reads no texts"
    );
    open_team_view(&mut window, &mut view, 701);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            label_centre(&nodes, "Support range"),
            "spread"
        ),
        None,
        "no game tooltip on PES 2016"
    );
}

// PES 2014's stored offsets reach the body boxes' ranges: the Height box
// runs from the 132 the record stores less to the u8 field's own top, the
// Weight box's floor is the 30 less the record stores; a typed 200 and 90
// land, a typed 400 clamps to the top.
#[test]
fn a_pes14_save_shifts_the_height_and_weight_ranges() {
    let path = pes14_fixture_plain("pes14_ranges");
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
    let path = pes14_fixture_plain("pes14_look_gated");
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
    for label in ["Boots ID", "Gloves ID", "Sleeves", "Outfielder gloves"] {
        let dash = disabled_dash(&nodes, label);
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(dash), "stores no look"),
            Some("PES 2014 stores no look for this player yet: edit it in game first".to_string()),
            "{label}'s hover"
        );
    }
    // The physique and the record's edit flags grey with them — one
    // slider's dash and the Face flag's disabled box — and the Player
    // menu's FPC entry is off under the same "stores no look" tip.
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Neck length")
    });
    disabled_dash(&nodes, "Neck length");
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::CheckBox && node.label() == Some("Face")
    });
    assert!(
        node_labeled(&nodes, Role::CheckBox, "Face").is_disabled(),
        "the face flag's box is disabled on a record-less player"
    );
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Free kick")
    });
    assert!(
        !control_right_of(&nodes, "Free kick").is_disabled(),
        "the free kick motion is stored on PES 2014"
    );
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    let item = menu_item(&nodes, "Hide default body (Enable FPC)");
    assert!(
        item.is_disabled(),
        "the FPC entry is disabled on a record-less player"
    );
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(item), "stores no look"),
        Some("PES 2014 stores no look for this player yet: edit it in game first".to_string()),
        "the entry's hover"
    );
}

// Set boots and gloves IDs on PES 2014: a gloves choice sets the team's
// players' gloves as on the other versions — the IDs land in the
// appearance records' runs, read back from the saved file — and a filled
// slot whose player has no appearance record still refuses the whole
// operation naming him, as for the boots.
#[test]
fn set_boots_and_gloves_on_pes14_sets_gloves_and_refuses_record_less_players() {
    use crate::session::team_ops::{BootsMode, GlovesMode};

    let path = pes14_fixture_with_look("pes14_boglo");
    let mut view = SaveView::new();
    view.open(&path);
    view.session
        .as_mut()
        .unwrap()
        .set_boots_and_gloves(701, BootsMode::Same(83), GlovesMode::Same(11))
        .expect("the gloves choice sets on PES 2014");
    view.session
        .as_ref()
        .unwrap()
        .file()
        .save(&path)
        .expect("the save writes");
    let back = EditFile::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
    for slot in back
        .team(701)
        .expect("team 701")
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
    {
        let player = back.player(slot.player_id).expect("the player");
        assert_eq!(player.appearance.boots_id, 83, "{}'s boots", player.id);
        assert_eq!(player.appearance.gloves_id, 11, "{}'s gloves", player.id);
    }

    // The vanilla save's players hold no appearance records, so the same
    // operation refuses on the first slot's player as for the boots alone
    // before, naming him as the roster shows him, nothing written.
    let path = pes14_fixture_plain("pes14_boglo_refused");
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
    for gloves in [GlovesMode::Unchanged, GlovesMode::Same(1)] {
        let error = view
            .session
            .as_mut()
            .unwrap()
            .set_boots_and_gloves(701, BootsMode::Same(83), gloves)
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
    }
}

// The Strip style panel's Outfielder gloves row on a PES 2014 player the
// appearance record holds: enabled, its dropdown offers the game's Off
// and Auto, and a pick of Auto writes the record's own bit — 0x56:2, which
// the other versions keep in the in-game face run — and nothing else in
// the record.
#[test]
fn the_outfielder_gloves_row_writes_pes14s_own_bit() {
    use pes_savefile::container::SaveContainer;
    use pes_savefile::schema::schema_for;

    let path = pes14_fixture_with_look("pes14_outfielder_gloves");
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
    let player_id = selected_player(&view).expect("a player selected").2;
    open_appearance(&mut window, &mut view);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Outfielder gloves")
    });
    let combo = right_of_label(&nodes, "Outfielder gloves", Role::ComboBox)
        .first()
        .copied()
        .expect("the Outfielder gloves dropdown");
    assert!(!combo.is_disabled(), "the row is enabled on PES 2014");
    assert_eq!(
        combo.value(),
        Some("Off"),
        "premise: the row shows Off first"
    );
    let record_bytes = |path: &std::path::Path| {
        let container =
            SaveContainer::decrypt(&std::fs::read(path).unwrap()).expect("the save decrypts");
        let (layout, appearance) = schema_for(PesVersion::Pes14)
            .appearance
            .as_ref()
            .expect("the appearance section");
        let count = usize::from(u16::from_le_bytes(
            container.payload[layout.count_offset..layout.count_offset + 2]
                .try_into()
                .expect("u16"),
        ));
        (0..count)
            .map(|i| &container.payload[layout.offset + i * appearance.size..][..appearance.size])
            .find(|rec| u32::from_le_bytes(rec[..4].try_into().expect("id")) == player_id)
            .expect("the player's appearance record")
            .to_vec()
    };
    let before = record_bytes(&path);
    assert_eq!(before[0x56] & 0b100, 0, "premise: the bit starts clear");

    // The dropdown's pair is the game's Off and Auto.
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let gloves_combo = {
        let label = ids
            .iter()
            .find(|(_, node)| {
                node.role() == Role::Label && node.value() == Some("Outfielder gloves")
            })
            .map(|(_, node)| node.bounds().expect("the label's bounds"))
            .expect("the row's label");
        ids.iter()
            .find(|(_, node)| node.role() == Role::ComboBox && on_row_right_of(label, node))
            .map(|(id, _)| *id)
            .expect("the Outfielder gloves dropdown")
    };
    action_click(&mut window, &mut view, gloves_combo);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for item in ["Off", "Auto"] {
        item_id(&ids, item);
    }
    action_click(&mut window, &mut view, gloves_combo);
    pick_entry(&mut window, &mut view, gloves_combo, "Auto");

    view.session
        .as_ref()
        .unwrap()
        .file()
        .save(&path)
        .expect("the save writes");
    let after = record_bytes(&path);
    for (byte, (&was, &now)) in before.iter().zip(&after).enumerate() {
        let expected = if byte == 0x56 { was | 0b100 } else { was };
        assert_eq!(now, expected, "byte {byte:#04x}");
    }
}

// A gated strip cell's `-` fills the field slot as the enabled controls
// do: on the look-carrying fixture the gated strip colours' dashes stand
// as wide as the Boots ID box, and on the vanilla save the record-less
// player's Outfielder gloves dash fills the same slot.
#[test]
fn a_gated_strip_cells_dash_fills_the_field_slot_on_pes14() {
    let strip_nodes = |view: &mut SaveView, window: &mut Headless| {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        click(
            window,
            nodes
                .iter()
                .find(|node| player_row(node))
                .expect("a player row"),
            view,
        );
        open_appearance(window, view);
        scroll_into_view(window, view, |node| {
            node.role() == Role::Label && node.value() == Some("Boots ID")
        })
    };
    let width = |node: &accesskit::Node| {
        let bounds = node.bounds().expect("the control's bounds");
        bounds.x1 - bounds.x0
    };

    let path = pes14_fixture_with_look("pes14_dash_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let nodes = strip_nodes(&mut view, &mut window);
    let boots = width(
        right_of_label(&nodes, "Boots ID", Role::SpinButton)
            .first()
            .expect("the enabled Boots ID box"),
    );
    for label in ["Spectacles color", "Gloves color"] {
        let dash = width(disabled_dash(&nodes, label));
        assert!(
            (dash - boots).abs() <= 1.0,
            "the {label} stand-in {dash} is the {boots}-wide field slot's"
        );
    }

    let path = pes14_fixture_plain("pes14_dash_width_plain");
    view.open(&path);
    let nodes = strip_nodes(&mut view, &mut window);
    let dash = width(disabled_dash(&nodes, "Outfielder gloves"));
    assert!(
        (dash - 118.0).abs() <= 1.0,
        "the Outfielder gloves stand-in {dash} fills the field slot"
    );
}

/// The `(boots_id, gloves_id)` of `id`'s player as `path`'s saved file
/// reads them.
fn boglo_of(path: &std::path::Path, id: u32) -> (u32, u32) {
    let back = EditFile::from_bytes(&std::fs::read(path).unwrap()).unwrap();
    let player = back.player(id).expect("the player");
    (player.appearance.boots_id, player.appearance.gloves_id)
}

// The Player menu's FPC entry on PES 2014: enabled on a player the
// appearance record holds, its press writing the hide pair — boots 55
// and gloves 11, no skin, the version mapping no in-game face run —
// read back from the saved file; the entry then reads Show, whose
// press writes 0 and 0 back.
#[test]
fn the_player_menus_fpc_entry_hides_and_shows_on_pes14() {
    let path = pes14_fixture_with_look("pes14_fpc_menu");
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
    let player_id = selected_player(&view).expect("a player selected").2;
    let nodes = open_menu(&mut window, &mut view, "Player", "Hide default body");
    let item = menu_item(&nodes, "Hide default body (Enable FPC)");
    assert!(!item.is_disabled(), "the FPC entry is enabled on PES 2014");
    click(&mut window, item, &mut view);
    view.session
        .as_ref()
        .unwrap()
        .file()
        .save(&path)
        .expect("the save writes");
    assert_eq!(boglo_of(&path, player_id), (55, 11), "the hide pair");

    let nodes = open_menu(&mut window, &mut view, "Player", "Show default body");
    let item = menu_item(&nodes, "Show default body (Disable FPC)");
    click(&mut window, item, &mut view);
    view.session
        .as_ref()
        .unwrap()
        .file()
        .save(&path)
        .expect("the save writes");
    assert_eq!(boglo_of(&path, player_id), (0, 0), "the show pair");
}

// The Appearance page's Full Player Customization group on PES 2014:
// its Hide and Show buttons write as the Player menu's entry does — the
// hide pair on a player the appearance record holds, the saved file
// read back — and the reading flips to "currently hidden"; on a player
// the section holds no record for they keep the Player entry's own
// "stores no look" hover, no reading drawn.
#[test]
fn the_fpc_group_writes_and_reads_the_hide_pair_on_pes14() {
    let except = {
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
        file.team(1)
            .expect("the vanilla first team")
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .nth(1)
            .expect("a second filled slot")
            .player_id
    };
    let path = pes14_fixture_with_look_missing("pes14_fpc_group", &[except]);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let player_id = {
        click(
            &mut window,
            nodes
                .iter()
                .find(|node| player_row(node))
                .expect("a player row"),
            &mut view,
        );
        selected_player(&view).expect("a player selected").2
    };
    open_appearance(&mut window, &mut view);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Button && node.label() == Some("Hide default body")
    });
    let hide = node_labeled(&nodes, Role::Button, "Hide default body");
    assert!(
        !hide.is_disabled(),
        "the group's Hide button is enabled on PES 2014"
    );
    click(&mut window, hide, &mut view);
    view.session
        .as_ref()
        .unwrap()
        .file()
        .save(&path)
        .expect("the save writes");
    assert_eq!(boglo_of(&path, player_id), (55, 11), "the hide pair");
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Button && node.label() == Some("Show default body")
    });
    assert!(
        label_value(&nodes, "The default body is currently hidden"),
        "the reading flips to hidden"
    );

    // The record-less player's: both buttons disabled under the look
    // controls' own tip, no reading drawn.
    view.selected = Some(Selection::Player(
        701,
        team_entry(&view, 701)
            .roster
            .iter()
            .position(|slot| slot.player_id == except)
            .map(|slot| u8::try_from(slot).expect("a roster slot fits u8"))
            .expect("the record-less player is rostered"),
    ));
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Button && node.label() == Some("Hide default body")
    });
    for label in ["Hide default body", "Show default body"] {
        let button = node_labeled(&nodes, Role::Button, label);
        assert!(
            button.is_disabled(),
            "{label} is disabled on a record-less player"
        );
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(button), "stores no look"),
            Some("PES 2014 stores no look for this player yet: edit it in game first".to_string()),
            "{label}'s hover"
        );
    }
    assert!(
        !label_value(&nodes, "The default body is currently hidden")
            && !label_value(&nodes, "The default body is currently shown"),
        "no reading on a record-less player"
    );
}

// The Team menu's Hide item on PES 2014: every team player the
// appearance section holds a record for takes the hide pair, read back
// from the saved file; a player without a record keeps his look.
#[test]
fn the_team_menus_hide_writes_the_pair_where_a_record_exists() {
    let except = {
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
        crate::view::copy_stats::first_filled(&file, 1).expect("a filled slot")
    };
    let path = pes14_fixture_with_look_missing("pes14_fpc_team", &[except]);
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
    let nodes = open_menu(&mut window, &mut view, "Team", "Hide default body");
    let item = menu_item(&nodes, "Hide default body (Enable FPC)");
    assert!(!item.is_disabled(), "the Hide item is enabled on PES 2014");
    click(&mut window, item, &mut view);
    view.session
        .as_ref()
        .unwrap()
        .file()
        .save(&path)
        .expect("the save writes");
    let back = EditFile::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
    for slot in back
        .team(701)
        .expect("team 701")
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
    {
        if slot.player_id == except {
            assert!(!back.has_appearance_record(except), "still no record");
            assert_eq!(boglo_of(&path, except), (0, 0), "his look is unchanged");
        } else {
            assert_eq!(
                boglo_of(&path, slot.player_id),
                (55, 11),
                "{}'s hide pair",
                slot.player_id
            );
        }
    }

    // Show writes them back: (0, 0) on every slot holding a record.
    let nodes = open_menu(&mut window, &mut view, "Team", "Show default body");
    click(
        &mut window,
        menu_item(&nodes, "Show default body (Disable FPC)"),
        &mut view,
    );
    view.session
        .as_ref()
        .unwrap()
        .file()
        .save(&path)
        .expect("the save writes");
    for slot in back
        .team(701)
        .expect("team 701")
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
    {
        if slot.player_id != except {
            assert_eq!(
                boglo_of(&path, slot.player_id),
                (0, 0),
                "{}'s show pair",
                slot.player_id
            );
        }
    }
    assert_eq!(boglo_of(&path, except), (0, 0), "his look stays his own");
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

// PES 2014 stores a punt kick and no corner kick, so the Motion group's
// Corner kick cell draws Punt kick on a PES 2014 save: enabled as a
// stored motion, its box storing one below the shown value, and Randomize
// draws it with the rest.
#[test]
fn a_pes14_save_draws_punt_kick_in_the_corner_kick_cell() {
    let path = pes14_fixture_plain("pes14_punt");
    let mut view = SaveView::new();
    view.open(&path);
    view.randomize_draw = Box::new(|top| top);
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
    let player_id = selected_player(&view).expect("a player selected").2;
    let punt_kick = |view: &SaveView| player_entry(view, player_id).motion.punt_kick;
    assert_ne!(punt_kick(&view), Some(1), "premise: not already 1 stored");

    // The cell draws Punt kick enabled and no Corner kick at all.
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Punt kick")
    });
    assert!(
        !control_right_of(&nodes, "Punt kick").is_disabled(),
        "the punt kick motion is stored on PES 2014"
    );
    assert!(
        !nodes
            .iter()
            .any(|node| node.role() == Role::Label && node.value() == Some("Corner kick")),
        "PES 2014 stores no corner kick"
    );

    // A 2 typed into the box stores 1, the motion shown from 1.
    let punt = right_of_label(&nodes, "Punt kick", Role::SpinButton)
        .first()
        .copied()
        .expect("the Punt kick control");
    click(&mut window, punt, &mut view);
    type_text(&mut window, &mut view, "2");
    assert_eq!(punt_kick(&view), Some(1), "shown 2 stores 1");

    // Randomize draws the punt kick too: the pinned source lands it at
    // its shown top of 4, stored 3.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (button, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Randomize"))
        .expect("the Randomize button");
    action_click(&mut window, &mut view, *button);
    assert_eq!(
        punt_kick(&view),
        Some(3),
        "Randomize stores the shown top less 1"
    );
}

// The cell is the corner kick's on every other version: a PES 2015 save's
// Motion group draws Corner kick and no Punt kick, the version storing
// none.
#[test]
fn a_pes15_save_keeps_corner_kick_in_the_cell() {
    let path = fixture(PesVersion::Pes15, "pes15_corner");
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
        node.role() == Role::Label && node.value() == Some("Corner kick")
    });
    assert!(
        !nodes
            .iter()
            .any(|node| node.role() == Role::Label && node.value() == Some("Punt kick")),
        "PES 2015 stores no punt kick"
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

// A PES 2014 badge holds the starter's overall at his slot's position by
// PES 2014's own formula, as the game's "Position / Overall Rating" does:
// J. WALTERS (42810), IRELAND's starter 8, fields at LMF, which he is
// rated A at — each ability less 25, times its weight: dribbling 77
// less 25 = 52, x .26, + speed 56 x .26 + explosive power 51 x .23 +
// ball control 49 x .16 + stamina 60 x .14 + lofted pass 40 x .13 +
// attacking prowess 55 x .07 + low pass 45 x .07 + swerve 40 x .04
// = 13.52 + 14.56 + 11.73 + 7.84 + 8.4 + 5.2 + 3.85 + 3.15 +
// 1.6 = 69.85, floor(69.85 + .5) = 70, plus 5 for the A: 75.
#[test]
fn a_pes14_badge_shows_the_starters_overall_by_pes14s_formula() {
    let path = pes14_fixture_plain("pes14_badge_overall");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(1600.0, 1400.0);

    assert_eq!(
        player_entry(&view, 42810).overall_at(PesVersion::Pes14, 6),
        Some(75),
        "premise: the formula's 75 at LMF"
    );
    view.selected = Some(Selection::Team(701));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let canvas = canvas_of(&nodes);
    let players = team_entry(&view, 701).tactics.presets[0].formations[0].players;
    assert_eq!(players[8].position, 6, "premise: starter 8 fields at LMF");
    let badge =
        egui::Rect::from_center_size(clamped_centre(canvas, players[8]), egui::vec2(50.0, 14.0));
    let number = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if (shape.pos.x + shape.galley.size().x - (badge.right() - 2.0)).abs() <= 0.5
                    && (shape.pos.y + shape.galley.size().y / 2.0 - badge.center().y).abs()
                        <= 0.5 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .expect("a number at starter 8's badge's right end");
    assert_eq!(number.galley.job.text, "75", "starter 8's overall at LMF");
}

// A rename on PES 2014 reaches the record's name flag, the bit the game
// reads a stored name by: byte 0x0A's bit 0, clear on the untouched
// fixture.
#[test]
fn a_pes14_team_rename_writes_the_name_flag() {
    use pes_savefile::container::SaveContainer;
    use pes_savefile::schema::schema_for;

    /// Team 701's record bytes in `save`'s decrypted payload.
    fn team_record(save: &[u8]) -> Vec<u8> {
        let container = SaveContainer::decrypt(save).expect("the save decrypts");
        let schema = schema_for(PesVersion::Pes14);
        let count = u16::from_le_bytes(
            container.payload[schema.teams.count_offset..schema.teams.count_offset + 2]
                .try_into()
                .expect("u16"),
        ) as usize;
        (0..count)
            .map(|i| {
                container.payload[schema.teams.offset + i * schema.team.size
                    ..schema.teams.offset + (i + 1) * schema.team.size]
                    .to_vec()
            })
            .find(|rec| u16::from_le_bytes(rec[..2].try_into().expect("id")) == 701)
            .expect("team 701's record")
    }

    let path = pes14_fixture_plain("pes14_rename_flag");
    assert_eq!(
        team_record(&std::fs::read(&path).unwrap())[0x0A] & 1,
        0,
        "premise: the fixture's name flag is clear"
    );

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name = display_name(&team_entry(&view, 701).name);
    let field = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some(name.as_str()))
        .expect("the team's Name field");
    click(&mut window, field, &mut view);
    type_text(&mut window, &mut view, "RENAMED");
    assert!(
        team_entry(&view, 701).edit_flags.name,
        "the rename sets the name flag"
    );
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });

    assert_eq!(
        team_record(&std::fs::read(&path).unwrap())[0x0A] & 1,
        1,
        "the written save's team record holds 0x0A:0"
    );
}
