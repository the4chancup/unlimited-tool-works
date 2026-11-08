use super::*;

// SE-GUI-23
#[test]
fn the_team_menu_operations_edit_every_filled_slot_as_one_undoable_edit() {
    let path = fixture(PesVersion::Pes21, "team_menu_ops");
    // A goalkeeper among the CB placeholders, so the position names cover
    // two table entries; the visual flags set, so Clear visual flags is a
    // real change rather than a no-op batch.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().positions.registered = 0;
    let ids: Vec<u32> = file
        .team(701)
        .unwrap()
        .roster
        .iter()
        .map(|slot| slot.player_id)
        .filter(|&id| id != 0)
        .collect();
    for id in ids {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    // A rostered player of another team, to prove the operations stay on
    // the selected player's team.
    let other_id = {
        let file = view.session.as_ref().unwrap().file();
        file.teams()
            .iter()
            .filter(|team| team.id != 701)
            .flat_map(|team| team.roster.iter())
            .map(|slot| slot.player_id)
            .find(|&id| id != 0)
            .expect("a player of another team")
    };
    let other_before = player_entry(&view, other_id);
    assert!(!player_entry(&view, 70101).edit_flags.player, "premise");

    // Set edited flag: every filled slot's player flag is set.
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set edited flag"),
        &mut view,
    );
    let slots = rostered(&view, 701);
    assert!(slots.len() > 2, "premise: several filled slots");
    for &(_, id) in &slots {
        assert!(player_entry(&view, id).edit_flags.player, "{id} flagged");
    }
    assert_eq!(player_entry(&view, other_id), other_before, "untouched");

    // Set boots and gloves IDs for everyone: incremental from 100, gloves
    // like the boot IDs — slot k gets 100 + k - 1 on both (the check's
    // bound is the record's 14-bit field, not settings.toml's 0 to 100).
    let start_value = 100u32;
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(
            &nodes,
            Role::Button,
            "Set boots and gloves IDs for everyone\u{2026}",
        ),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let incremental = node_labeled(&nodes, Role::RadioButton, "Set incremental value:");
    click(&mut window, incremental, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let incremental = node_labeled(&nodes, Role::RadioButton, "Set incremental value:");
    let start = right_of(&nodes, incremental, Role::SpinButton);
    click(&mut window, start, &mut view);
    window.frame(vec![egui::Event::Text(start_value.to_string())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::RadioButton, "Set like boot IDs"),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set"),
        &mut view,
    );
    for &(slot, id) in &slots {
        let player = player_entry(&view, id);
        let expected = start_value + u32::try_from(slot).expect("a roster has at most 40 slots");
        assert_eq!(
            player.appearance.boots_id,
            expected,
            "slot {}'s boots",
            slot + 1
        );
        assert_eq!(
            player.appearance.gloves_id,
            expected,
            "slot {}'s gloves",
            slot + 1
        );
    }

    // Set player names to positions: the registered position's full name.
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set player names to positions"),
        &mut view,
    );
    for &(slot, id) in &slots {
        let player = player_entry(&view, id);
        assert_eq!(
            player.name,
            POSITION_NAMES[usize::from(player.positions.registered)],
            "slot {}'s name",
            slot + 1
        );
    }
    assert_eq!(player_entry(&view, 70101).name, "Goalkeeper");
    assert_eq!(player_entry(&view, 70102).name, "Center Back");

    // Clear visual flags by Ctrl+F: the four flags off on every slot.
    window.frame(vec![key(egui::Key::F, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &(_, id) in &slots {
        let flags = &player_entry(&view, id).edit_flags;
        assert!(
            !(flags.face || flags.hair || flags.physique || flags.strip),
            "{id}'s visual flags cleared"
        );
    }

    // Four operations, four undo entries; the undos put every player back.
    assert_eq!(undo_entries(&mut window, &mut view), 4, "one entry each");
    assert_eq!(player_entry(&view, 70101).name, "PLACEHOLDER");
    assert!(!player_entry(&view, 70101).edit_flags.player);
    // Redo them all, save, reopen: the file holds the operation's values.
    for _ in 0..4 {
        window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
    }
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    for &(slot, id) in &slots {
        let player = reopened.player(id).expect("the player");
        let expected = start_value + u32::try_from(slot).expect("a roster has at most 40 slots");
        assert_eq!(player.appearance.boots_id, expected);
        assert_eq!(player.appearance.gloves_id, expected);
        assert_eq!(
            player.name,
            POSITION_NAMES[usize::from(player.positions.registered)]
        );
        assert!(player.edit_flags.player);
        let flags = &player.edit_flags;
        assert!(!(flags.face || flags.hair || flags.physique || flags.strip));
    }
    assert_eq!(
        reopened.player(other_id).unwrap(),
        &other_before,
        "a player of another team is unchanged"
    );
}

// SE-GUI-22 (the Team menu's part): under the lock the aesthetics items
// are disabled with the ownership line as hover, Ctrl+F is a no-op, and
// the gameplay items still run.
#[test]
fn the_locked_team_menu_disables_the_aesthetics_items_with_the_ownership_line() {
    let path = fixture(PesVersion::Pes21, "team_menu_locked");
    // Two players' visual flags set, so a guardless Ctrl+F would make a
    // visible change (the fixture's are clear, which let the guard go).
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    for id in [70101u32, 70102] {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = open_team_menu(&mut window, &mut view);

    for label in [
        "Set boots and gloves IDs for everyone\u{2026}",
        "Clear visual flags",
    ] {
        let item = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Button
                    && node
                        .label()
                        .is_some_and(|node_label| node_label.starts_with(label))
            })
            .unwrap_or_else(|| panic!("no menu item {label:?}"));
        assert!(item.is_disabled(), "{label} is disabled under the lock");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(item), "The team's export"),
            Some(
                "The team's export owns the aesthetics, so edit its settings.toml (the face, hair and iris: its ingame_face.toml) and recompile, then apply the patch."
                    .to_string(),
            ),
            "{label}'s hover is the ownership line"
        );
    }
    for label in ["Set edited flag", "Set player names to positions"] {
        assert!(
            !node_labeled(&nodes, Role::Button, label).is_disabled(),
            "{label} stays enabled"
        );
    }

    // Ctrl+F under the lock: nothing changes, the flags still set.
    window.frame(vec![key(egui::Key::F, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        !view.session.as_ref().unwrap().is_dirty(),
        "Ctrl+F changed nothing"
    );
    for id in [70101u32, 70102] {
        let flags = &player_entry(&view, id).edit_flags;
        assert!(
            flags.face && flags.hair && flags.physique && flags.strip,
            "{id}'s visual flags survived the locked Ctrl+F"
        );
    }

    // Set edited flag still runs.
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set edited flag"),
        &mut view,
    );
    for &(_, id) in &rostered(&view, 701) {
        assert!(player_entry(&view, id).edit_flags.player, "{id} flagged");
    }
}

// The menu item itself (the other SE-GUI-23 test runs the clear through
// Ctrl+F). Every filled slot's four visual flags set first, then cleared,
// one undo restoring every one of them, redo clearing them again, and the
// saved file holding them clear.
// SE-GUI-23
#[test]
fn the_clear_visual_flags_item_clears_every_slot_with_undo_redo_and_save() {
    let path = fixture(PesVersion::Pes21, "team_menu_clear_flags");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let filled: Vec<u32> = file
        .team(701)
        .unwrap()
        .roster
        .iter()
        .map(|slot| slot.player_id)
        .filter(|&id| id != 0)
        .collect();
    for &id in &filled {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let cleared = |view: &SaveView, id: u32| {
        let flags = &player_entry(view, id).edit_flags;
        !(flags.face || flags.hair || flags.physique || flags.strip)
    };
    let all_set = |view: &SaveView, id: u32| {
        let flags = &player_entry(view, id).edit_flags;
        flags.face && flags.hair && flags.physique && flags.strip
    };

    let nodes = open_team_menu(&mut window, &mut view);
    let item = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Clear visual flags"))
        })
        .expect("the Clear visual flags item");
    assert!(!item.is_disabled(), "unlocked, the item is enabled");
    click(&mut window, item, &mut view);
    let slots = rostered(&view, 701);
    for &(_, id) in &slots {
        assert!(cleared(&view, id), "{id}'s visual flags cleared");
    }

    // One undo entry restores them; redo clears them again.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    for &id in &filled {
        assert!(all_set(&view, id), "{id}'s four flags restored");
    }
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &(_, id) in &slots {
        assert!(cleared(&view, id), "{id}'s flags cleared again");
    }

    // Save; the reopened file holds them cleared.
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    for &(_, id) in &slots {
        let flags = &reopened.player(id).unwrap().edit_flags;
        assert!(
            !(flags.face || flags.hair || flags.physique || flags.strip),
            "{id}'s flags cleared in the saved file"
        );
    }
}

// Each Team menu operation acts on the selected row's team, a team row's
// included — Clear visual flags by the menu item and by Ctrl+F, each in
// turn from the same state; the menu is disabled only with no row
// selected.
#[test]
fn the_team_menu_and_ctrl_f_act_on_the_selected_rows_team() {
    let path = fixture(PesVersion::Pes21, "team_menu_on_team_row");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let filled: Vec<u32> = file
        .team(701)
        .unwrap()
        .roster
        .iter()
        .map(|slot| slot.player_id)
        .filter(|&id| id != 0)
        .collect();
    for &id in &filled {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let cleared = |view: &SaveView, id: u32| {
        let flags = &player_entry(view, id).edit_flags;
        !(flags.face || flags.hair || flags.physique || flags.strip)
    };
    let all_set = |view: &SaveView, id: u32| {
        let flags = &player_entry(view, id).edit_flags;
        flags.face && flags.hair && flags.physique && flags.strip
    };

    // The team row selected: the menu item is enabled and clears.
    open_team_view(&mut window, &mut view, 701);
    let nodes = open_team_menu(&mut window, &mut view);
    let item = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Clear visual flags"))
        })
        .expect("the Clear visual flags item");
    assert!(!item.is_disabled(), "the item is enabled on a team row");
    click(&mut window, item, &mut view);
    for &(_, id) in &rostered(&view, 701) {
        assert!(cleared(&view, id), "{id}'s visual flags cleared");
    }

    // Ctrl+Z restores them; from the same state Ctrl+F clears the team's
    // flags as it does from a player row.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &id in &filled {
        assert!(all_set(&view, id), "{id}'s flags restored");
    }
    window.frame(vec![key(egui::Key::F, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &(_, id) in &rostered(&view, 701) {
        assert!(cleared(&view, id), "{id}'s visual flags cleared by Ctrl+F");
    }

    // With no row selected the menu is disabled.
    view.selected = None;
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        menu_button(&nodes, "Team").is_disabled(),
        "the Team menu is disabled with no row selected"
    );
}
