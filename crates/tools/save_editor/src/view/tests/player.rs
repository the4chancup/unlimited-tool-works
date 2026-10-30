//! The Player menu's stat operations and the FPC reads and writes, end to
//! end through the headless window: SE-GUI-24's four operations,
//! SE-GUI-11's hide/show, the SE-GUI-22 lock's FPC items, and the PES
//! 15/16 appearance record's round-trip.

use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::PlayerEntry;
use pes_savefile::model::names::display_name;

use super::*;

/// The rostered `(slot, player_id)` pairs of `team_id`, in slot order.
fn rostered(view: &SaveView, team_id: u32) -> Vec<(usize, u32)> {
    team_entry(view, team_id)
        .roster
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.player_id != 0)
        .map(|(slot, roster_slot)| (slot, roster_slot.player_id))
        .collect()
}

/// The number dialog's value field: the nearest number box under the
/// sentence `line`, sharing its left edge.
fn field_under<'a>(nodes: &'a [accesskit::Node], line: &str) -> &'a accesskit::Node {
    let line_bounds = label_bounds(nodes, line);
    nodes
        .iter()
        .filter(|node| {
            node.role() == Role::SpinButton
                && node.bounds().is_some_and(|b| {
                    b.y0 >= line_bounds.y1 - 1.0 && (b.x0 - line_bounds.x0).abs() <= 1.0
                })
        })
        .min_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().expect("bounds").y0;
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .unwrap_or_else(|| panic!("no number box under {line:?}"))
}

/// Opens the number dialog of `item` and types `value` into it, then OK.
fn run_stat_dialog(
    window: &mut Headless,
    view: &mut SaveView,
    item: &str,
    line: &str,
    value: &str,
) {
    let nodes = open_menu(window, view, "Player", "Set all stats");
    click(window, menu_item(&nodes, item), view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let field = field_under(&nodes, line);
    click(window, field, view);
    type_text(window, view, value);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    click(window, node_labeled(&nodes, Role::Button, "OK"), view);
}

/// The line `stat_num` draws for `name`, as the dialog's field anchors on it.
fn set_line(name: &str) -> String {
    format!("Set all of {name}'s ability stats to")
}

/// The Bump dialog's line for `name`.
fn bump_line(name: &str) -> String {
    format!("Add to all of {name}'s ability stats, clamped to 40 to 99")
}

/// Every ability of `player` is `value` (the ones the version has).
fn assert_abilities(player: &PlayerEntry, value: u8) {
    for (index, ability) in player.stats.abilities().iter().enumerate() {
        if let Some(ability) = ability {
            assert_eq!(*ability, value, "ability {index}");
        }
    }
}

// The Set and Bump rows: Set all stats writes every ability the
// version has and nothing else (no edit flag, weak foot, form and injury
// untouched); Bump adds N clamped to 40 to 99, the dialog remembered and N
// read whole (a -10 is a bump of -10, not -1); each one undoable step.
// SE-GUI-24
#[test]
fn set_and_bump_stats_write_every_ability_as_one_undoable_edit() {
    let path = fixture(PesVersion::Pes21, "player_set_bump");
    // 70102's first ability off the fixture's, for the negative bump.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70102).unwrap().stats.attacking_prowess = 60;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let before = player_entry(&view, 70101);
    assert_ne!(
        before.stats.attacking_prowess, 75,
        "premise: not already 75"
    );

    // Set all stats to 75 through the dialog (the field is prefilled with
    // his Attacking Prowess, the first ability).
    let name = display_name(&before.name);
    run_stat_dialog(
        &mut window,
        &mut view,
        "Set all stats",
        &set_line(&name),
        "75",
    );
    let set = player_entry(&view, 70101);
    assert_abilities(&set, 75);
    assert_eq!(set.stats.weak_foot_usage, before.stats.weak_foot_usage);
    assert_eq!(
        set.stats.weak_foot_accuracy,
        before.stats.weak_foot_accuracy
    );
    assert_eq!(set.stats.form, before.stats.form, "form is no ability");
    assert_eq!(
        set.stats.injury_resistance, before.stats.injury_resistance,
        "injury is no ability"
    );
    assert_eq!(set.edit_flags, before.edit_flags, "no edit flag is set");

    // Set to 90, then bump +10 by Ctrl+B: every ability clamps to 99 (the
    // dialog's N runs -10 to 10, so the overshoot is what clamps).
    run_stat_dialog(
        &mut window,
        &mut view,
        "Set all stats",
        &set_line(&name),
        "90",
    );
    window.frame(vec![key(egui::Key::B, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = field_under(&nodes, &bump_line(&name));
    click(&mut window, field, &mut view);
    type_text(&mut window, &mut view, "10");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "OK"),
        &mut view,
    );
    assert_abilities(&player_entry(&view, 70101), 99);

    // Set to 45, bump -10 through the menu: every ability clamps to 40.
    run_stat_dialog(
        &mut window,
        &mut view,
        "Set all stats",
        &set_line(&name),
        "45",
    );
    run_stat_dialog(
        &mut window,
        &mut view,
        "Bump all stats",
        &bump_line(&name),
        "-10",
    );
    assert_abilities(&player_entry(&view, 70101), 40);
    let bumped = player_entry(&view, 70101);
    assert_eq!(bumped.stats.form, before.stats.form);
    assert_eq!(
        bumped.stats.injury_resistance,
        before.stats.injury_resistance
    );
    assert_eq!(bumped.edit_flags, before.edit_flags);

    // Five operations, five undo entries; the undos restore the player.
    assert_eq!(undo_entries(&mut window, &mut view), 5, "one entry each");
    assert_eq!(player_entry(&view, 70101), before, "the undos restore all");

    // Redo them all, save, reopen: the file holds the 40s.
    for _ in 0..5 {
        window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
    }
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_abilities(EditFile::load(&path).unwrap().player(70101).unwrap(), 40);

    // N is read whole: a -10 on 70102's 60 gives 50, not the 59 a
    // two-character read would leave.
    view.selected = Some(Selection::Player(701, 1));
    let second_line = bump_line(&display_name(&player_entry(&view, 70102).name));
    run_stat_dialog(
        &mut window,
        &mut view,
        "Bump all stats",
        &second_line,
        "-10",
    );
    assert_eq!(
        player_entry(&view, 70102).stats.attacking_prowess,
        50,
        "-10 bumps by -10, not by -1"
    );
}

// The Set dialog's own clamp: the field's 40 to 99 range settles a typed
// 100 at 99 and a typed 30 at 40 (SE-GUI-24's "Set and Bump clamped to
// 40 to 99").
// SE-GUI-24
#[test]
fn the_set_dialog_clamps_the_typed_value_to_40_and_99() {
    let path = fixture(PesVersion::Pes21, "player_set_clamp");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let name = display_name(&player_entry(&view, 70101).name);
    for (typed, clamped) in [("100", 99u8), ("30", 40u8)] {
        run_stat_dialog(
            &mut window,
            &mut view,
            "Set all stats",
            &set_line(&name),
            typed,
        );
        assert_abilities(&player_entry(&view, 70101), clamped);
    }
}

// The Bump dialog's field prefills with the last N used in the window
// (`view.bump`): after a bump by 7 the reopened dialog's field shows 7.
// SE-GUI-24
#[test]
fn the_bump_dialog_prefills_the_last_n_used() {
    let path = fixture(PesVersion::Pes21, "player_bump_remember");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let name = display_name(&player_entry(&view, 70101).name);

    run_stat_dialog(
        &mut window,
        &mut view,
        "Bump all stats",
        &bump_line(&name),
        "7",
    );

    // Reopen the dialog: the field carries the last N, before anything
    // is typed.
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    click(&mut window, menu_item(&nodes, "Bump all stats"), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = field_under(&nodes, &bump_line(&name));
    assert_eq!(field.value(), Some("7"), "the last bump is prefilled");
}

// The number dialog draws as the draft's `dlgNum`: the sentence on its own
// line, the number box 8 points under it (the draft's `.window p` margin) at
// the body's left (the sentence's left edge, where the modal's content starts).
#[test]
fn the_set_dialogs_number_box_stands_under_its_sentence() {
    let path = fixture(PesVersion::Pes21, "player_set_layout");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let name = display_name(&player_entry(&view, 70101).name);
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    click(&mut window, menu_item(&nodes, "Set all stats"), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The page behind the modal draws number boxes of its own, so every
    // node is looked up inside the modal's area.
    let dialog = window
        .ctx
        .memory(|memory| memory.area_rect(egui::Id::unique("save_editor_stat_num")))
        .expect("the dialog's area");
    let inside = |node: &&accesskit::Node| {
        node.bounds().is_some_and(|b| {
            dialog.contains(egui::pos2(b.x0 as f32, b.y0 as f32))
                && dialog.contains(egui::pos2(b.x1 as f32, b.y1 as f32))
        })
    };
    let bounds_of = |role: Role, value: Option<&str>| {
        nodes
            .iter()
            .filter(inside)
            .find(|node| node.role() == role && (value.is_none() || node.value() == value))
            .and_then(accesskit::Node::bounds)
            .unwrap_or_else(|| panic!("no {role:?} {value:?} in the dialog"))
    };
    let line = bounds_of(Role::Label, Some(&set_line(&name)));
    // The page's own number boxes lie under the modal's area too, and the
    // nodes come in no set order: the dialog's is the nearest under the
    // sentence.
    let field = nodes
        .iter()
        .filter(inside)
        .filter(|node| node.role() == Role::SpinButton)
        .filter_map(accesskit::Node::bounds)
        .filter(|b| b.y0 >= line.y1 - 1.0)
        .min_by(|a, b| a.y0.total_cmp(&b.y0))
        .expect("a number box under the sentence");
    assert!(
        (field.y0 - (line.y1 + 8.0)).abs() <= 1.0,
        "the box's top {} is 8 under the sentence's bottom {}",
        field.y0,
        line.y1
    );
    assert!(
        (field.x0 - line.x0).abs() <= 1.0,
        "the box's left {} is the body's left {}",
        field.x0,
        line.x0
    );
}

/// `entry` with the copy/swap field set taken from `source`: height,
/// weight, the whole stats, positions and skills, the free-kick motion
/// alone, and the eight gameplay edit flags: Copy player stats' field
/// set.
fn with_stats_of(entry: &PlayerEntry, source: &PlayerEntry) -> PlayerEntry {
    let mut entry = entry.clone();
    entry.basic.height = source.basic.height;
    entry.basic.weight = source.basic.weight;
    entry.stats = source.stats.clone();
    entry.positions = source.positions.clone();
    entry.skills = source.skills.clone();
    entry.motion.free_kick = source.motion.free_kick;
    entry.edit_flags.player = source.edit_flags.player;
    entry.edit_flags.basic_settings = source.edit_flags.basic_settings;
    entry.edit_flags.registered_position = source.edit_flags.registered_position;
    entry.edit_flags.playable_positions = source.edit_flags.playable_positions;
    entry.edit_flags.abilities = source.edit_flags.abilities;
    entry.edit_flags.skills = source.edit_flags.skills;
    entry.edit_flags.playing_style = source.edit_flags.playing_style;
    entry.edit_flags.com_styles = source.edit_flags.com_styles;
    entry
}

/// Gives 70101 and 70102 distinct values in and out of the field set, so a
/// copy's kept half is not vacuous (`version`'s fixture written to `path`).
fn mark_copy_players(path: &std::path::Path, version: PesVersion) {
    let mut file = EditFile::from_bytes(&fixture_save(version)).unwrap();
    {
        let dest = file.player_mut(70101).unwrap();
        dest.name = "DEST".to_owned();
        dest.shirt_name = "DSHIRT".to_owned();
        dest.basic.nationality = 7;
        dest.basic.age = 25;
        dest.basic.height = 170;
        dest.basic.weight = 61;
        dest.motion.penalty_kick = 2;
        dest.motion.goal_celebration_1 = 30;
        // The kept flags all held on the destination, clear on the source:
        // a wrong copy shows.
        dest.edit_flags.motion = true;
        dest.edit_flags.face = true;
        dest.edit_flags.hair = true;
        dest.edit_flags.physique = true;
        dest.edit_flags.strip = true;
        dest.edit_flags.base_copy = dest.edit_flags.base_copy.map(|_| true);
        dest.appearance.boots_id = 33;
        dest.appearance.inners = 1;
    }
    {
        let source = file.player_mut(70102).unwrap();
        source.name = "SOURCE".to_owned();
        source.shirt_name = "SSHIRT".to_owned();
        source.basic.nationality = 99;
        source.basic.age = 40;
        source.basic.height = 191;
        source.basic.weight = 90;
        source.stats.speed = 88;
        source.stats.finishing = 77;
        source.stats.form = 6;
        source.positions.registered = 3;
        source.positions.playing_style = 5;
        source.positions.stronger_foot = 1;
        source.positions.playable[0] = 2;
        source.skills.skills[3] = true;
        source.skills.com_styles[2] = true;
        source.motion.free_kick = 9;
        source.motion.penalty_kick = 3;
        source.motion.goal_celebration_1 = 50;
        let flags = &mut source.edit_flags;
        flags.player = true;
        flags.basic_settings = true;
        flags.registered_position = true;
        flags.playable_positions = true;
        flags.abilities = true;
        flags.skills = true;
        flags.playing_style = true;
        flags.com_styles = true;
        // The kept flags explicitly clear, opposite of the destination's.
        flags.motion = false;
        flags.face = false;
        flags.hair = false;
        flags.physique = false;
        flags.strip = false;
        flags.base_copy = flags.base_copy.map(|_| false);
        source.appearance.boots_id = 44;
        source.appearance.inners = 2;
    }
    file.save(path).unwrap();
}

/// `slot`'s label in the Copy/Swap dialog's player combo: the shirt
/// number, registered position and display name, as the player list shows
/// them.
fn slot_label(entry: &PlayerEntry, number: u16) -> String {
    let position = POSITIONS
        .get(usize::from(entry.positions.registered))
        .copied()
        .unwrap_or("?");
    format!("{number} {position} {}", display_name(&entry.name))
}

/// The `role` control named by `label` inside a dialog: `right_of_label`,
/// except the label is picked among its copies (the team view's "Team"
/// and the player view's "Player" scope carry the same texts) by which
/// one's row holds a `role` node.
fn dialog_control<'a>(
    nodes: &'a [accesskit::Node],
    label: &str,
    role: Role,
) -> &'a accesskit::Node {
    nodes
        .iter()
        .filter(|node| node.role() == Role::Label && node.value() == Some(label))
        .find_map(|label_node| first_right_of(nodes, label_node, |n| n.role() == role))
        .unwrap_or_else(|| panic!("no {role:?} right of a {label:?} label"))
}

/// Picks `label` in the Copy/Swap dialog's player combo and clicks OK.
fn run_copy_swap(window: &mut Headless, view: &mut SaveView, item: &str, label: &str) {
    let nodes = open_menu(window, view, "Player", "Set all stats");
    click(window, menu_item(&nodes, item), view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let combo = dialog_control(&nodes, "Player", Role::ComboBox);
    click(window, combo, view);
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    // A list row can carry the same label, so the pick is scoped to the
    // open combo's popup — the `Role::ListBox` subtree egui builds for it.
    let parent: HashMap<accesskit::NodeId, accesskit::NodeId> = nodes
        .iter()
        .flat_map(|(id, node)| node.children().iter().map(move |child| (*child, *id)))
        .collect();
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        nodes.iter().map(|(id, node)| (*id, node)).collect();
    let in_listbox = |mut at: accesskit::NodeId| {
        for _ in 0..64 {
            if by_id[&at].role() == Role::ListBox {
                return true;
            }
            let Some(id) = parent.get(&at) else {
                return false;
            };
            at = *id;
        }
        panic!("a node deeper than 64")
    };
    let (option, _) = nodes
        .iter()
        .find(|(id, node)| node.label() == Some(label) && in_listbox(*id))
        .unwrap_or_else(|| panic!("no {label:?} in the player combo"));
    action_click(window, view, *option);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    click(window, node_labeled(&nodes, Role::Button, "OK"), view);
}

// The Copy row: the picked player's field set lands on the
// selected one and nothing outside it moves — names, nationality, age,
// every motion but the free kick's, the motion and appearance flags and
// the whole appearance stay — as one undoable edit, saved and reopened.
// SE-GUI-24
#[test]
fn copy_stats_writes_the_field_set_and_nothing_else() {
    let path = fixture(PesVersion::Pes21, "player_copy");
    mark_copy_players(&path, PesVersion::Pes21);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let dest_before = player_entry(&view, 70101);
    let source_before = player_entry(&view, 70102);
    // Premises: the fields a copy must and must not carry all differ.
    assert_ne!(dest_before.stats, source_before.stats);
    assert_ne!(dest_before.positions, source_before.positions);
    assert_ne!(dest_before.name, source_before.name);
    assert_ne!(dest_before.motion, source_before.motion);
    assert_ne!(dest_before.appearance, source_before.appearance);

    // The dialog's team combo opens on the selected player's team (701),
    // its player combo on the first filled slot: 70102 is the option named
    // by his shirt number, position and name.
    let number = team_entry(&view, 701).roster[1].number;
    run_copy_swap(
        &mut window,
        &mut view,
        "Copy stats from",
        &slot_label(&source_before, number),
    );

    assert_eq!(
        player_entry(&view, 70101),
        with_stats_of(&dest_before, &source_before),
        "the field set copied, the rest 70101's own"
    );
    assert_eq!(
        player_entry(&view, 70102),
        source_before,
        "the source is untouched"
    );
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(player_entry(&view, 70101), dest_before, "restored");

    // Redo, save, reopen: the file holds the copy.
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        EditFile::load(&path).unwrap().player(70101).unwrap(),
        &with_stats_of(&dest_before, &source_before),
        "the reopened file holds the copy"
    );
}

// Swap player stats: the same field set traded both ways between the
// selected and the picked player — here on PES 16, whose appearance block
// is a separate record and stays out of the trade, as the field set
// leaves it out.
// SE-GUI-24
#[test]
fn swap_stats_trades_the_field_set_on_pes16() {
    let path = fixture(PesVersion::Pes16, "player_swap16");
    mark_copy_players(&path, PesVersion::Pes16);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let first_before = player_entry(&view, 70101);
    let second_before = player_entry(&view, 70102);
    assert_ne!(first_before.appearance, second_before.appearance);

    let number = team_entry(&view, 701).roster[1].number;
    run_copy_swap(
        &mut window,
        &mut view,
        "Swap stats with",
        &slot_label(&second_before, number),
    );

    assert_eq!(
        player_entry(&view, 70101),
        with_stats_of(&first_before, &second_before),
        "70101 takes 70102's set, keeps the rest"
    );
    assert_eq!(
        player_entry(&view, 70102),
        with_stats_of(&second_before, &first_before),
        "70102 takes 70101's set, keeps the rest"
    );
    assert_eq!(
        player_entry(&view, 70101).appearance,
        first_before.appearance,
        "the appearance record stays"
    );
    assert_eq!(
        player_entry(&view, 70102).appearance,
        second_before.appearance,
        "the appearance record stays"
    );

    // One undo trades them back; redo trades again; saved file holds it.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(player_entry(&view, 70101), first_before);
    assert_eq!(player_entry(&view, 70102), second_before);
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    assert_eq!(
        reopened.player(70101).unwrap(),
        &with_stats_of(&first_before, &second_before)
    );
    assert_eq!(
        reopened.player(70102).unwrap(),
        &with_stats_of(&second_before, &first_before)
    );
}

// The four menu dialogs' headings are their titles (`StatNum::title`,
// `CopySwap::title`): the heading text is what a member reads the dialog
// by.
#[test]
fn the_menu_dialogs_draw_their_headings() {
    let path = fixture(PesVersion::Pes21, "player_dialog_headings");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    for (item, title) in [
        ("Set all stats", "Set all stats"),
        ("Bump all stats", "Bump all stats"),
        ("Copy stats from", "Copy stats"),
        ("Swap stats with", "Swap stats"),
    ] {
        let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
        click(&mut window, menu_item(&nodes, item), &mut view);
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert!(
            label_value(&nodes, title),
            "the {item:?} dialog draws its heading {title:?}"
        );
        click(
            &mut window,
            node_labeled(&nodes, Role::Button, "Cancel"),
            &mut view,
        );
    }
}

// The Copy/Swap dialog's pick: it opens on the selected player's team's
// first filled roster slot, and picking another team re-picks that
// team's first filled slot — the combo's shown text names the picked
// player by his number, position and name.
#[test]
fn the_copy_swap_dialog_picks_the_teams_first_filled_slot() {
    let path = fixture(PesVersion::Pes21, "player_copy_swap_pick");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    // The selected player is the first listed, so his team's first
    // filled slot is the pick. Another team of the fixture, for the
    // switch: read both from the save, never typed in.
    let (slot, first_id) = rostered(&view, 701)
        .first()
        .copied()
        .expect("team 701 has a filled slot");
    let first = player_entry(&view, first_id);
    let number = team_entry(&view, 701).roster[slot].number;
    let second_id = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .teams()
        .iter()
        .find(|team| team.id != 701 && team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("another team with a filled slot")
        .id;
    let second_name = team_entry(&view, second_id).name.clone();
    let (second_slot, second_player_id) = rostered(&view, second_id)
        .first()
        .copied()
        .expect("the other team has a filled slot");
    let second = player_entry(&view, second_player_id);
    let second_number = team_entry(&view, second_id).roster[second_slot].number;

    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    click(&mut window, menu_item(&nodes, "Copy stats from"), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        player_combo(&nodes).value().map(str::to_owned),
        Some(slot_label(&first, number)),
        "the pick is the team's first filled slot, shown by his label"
    );

    // Pick the other team in the Team combo: the pick re-seats on its
    // first filled slot.
    let team_combo = dialog_control(&nodes, "Team", Role::ComboBox);
    click(&mut window, team_combo, &mut view);
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (option, _) = nodes
        .iter()
        .find(|(_, node)| node.label() == Some(second_name.as_str()))
        .unwrap_or_else(|| panic!("no {second_name:?} in the team combo"));
    action_click(&mut window, &mut view, *option);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        player_combo(&nodes).value().map(str::to_owned),
        Some(slot_label(&second, second_number)),
        "the pick is the picked team's first filled slot"
    );
}

/// The Copy/Swap dialog's player combo node.
fn player_combo(nodes: &[accesskit::Node]) -> &accesskit::Node {
    dialog_control(nodes, "Player", Role::ComboBox)
}

// The Copy/Swap dialog leaves on OK as on Cancel (`view.copy_swap` is
// `None` after both), the `&&` mutant's keep-it-open case.
#[test]
fn the_copy_swap_dialog_closes_on_ok_and_cancel() {
    let path = fixture(PesVersion::Pes21, "player_copy_swap_close");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    for button in ["OK", "Cancel"] {
        let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
        click(&mut window, menu_item(&nodes, "Copy stats from"), &mut view);
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        click(
            &mut window,
            node_labeled(&nodes, Role::Button, button),
            &mut view,
        );
        assert!(view.copy_swap.is_none(), "{button} closed the dialog");
    }
}

/// The Button labeled `label` on the Appearance section's FPC group.
fn fpc_button<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    node_labeled(nodes, Role::Button, label)
}

/// The Appearance section unlocked, scrolled to the FPC group and drawn for
/// one frame's nodes. The group is the section's last: under the player
/// header it sits below the fold, and a pointer click or hover lands
/// only inside the scroll's clip (bounded: a group that never shows
/// fails rather than hangs).
fn appearance_frame(window: &mut Headless, view: &mut SaveView) -> Vec<accesskit::Node> {
    open_appearance(window, view);
    scroll_into_view(window, view, |node| {
        node.role() == Role::Button && node.label() == Some("Hide default body")
    })
}

/// 70101's strip fields the presets write, as the session holds him.
fn strip_of(view: &SaveView, id: u32) -> (u8, bool, u8, u32, u32) {
    let player = player_entry(view, id);
    (
        player.appearance.sleeves,
        player.appearance.untucked,
        player.appearance.socks,
        player.appearance.boots_id,
        player.appearance.gloves_id,
    )
}

// Hide default body gives the hide preset and the section reads "The
// default body is currently hidden"; Ctrl+I gives the show preset and
// "...shown". A
// model's own boots ID is kept and reads as shown. Set FPC invisibility
// hides every filled slot of the team in one undo step, and a strip
// setting that breaks hiding lists its finding on the section — reported,
// never cleared.
// SE-GUI-11
#[test]
fn hide_and_show_body_apply_the_presets_keeping_own_ids() {
    let path = fixture(PesVersion::Pes21, "fpc_hide_show");
    // 70101 dressed with inners on (the finding's case), 70102 with his
    // model's own boots ID.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    {
        let player = file.player_mut(70101).unwrap();
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
        player.appearance.sleeves = 1;
        player.appearance.untucked = true;
        player.appearance.socks = 0;
        player.appearance.inners = 1;
    }
    {
        let player = file.player_mut(70102).unwrap();
        player.appearance.boots_id = 627;
        player.appearance.gloves_id = 0;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    assert_eq!(
        strip_of(&view, 70101),
        (1, true, 0, 0, 0),
        "premise: dressed, boots and gloves 0"
    );

    // Hide default body: long sleeves, tucked, short socks, the nonexistent IDs,
    // the inners left on as the finding reports rather than clears.
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently shown"),
        "dressed reads shown"
    );
    click(
        &mut window,
        fpc_button(&nodes, "Hide default body"),
        &mut view,
    );
    assert_eq!(
        strip_of(&view, 70101),
        (2, false, 2, 55, 11),
        "the hide preset landed"
    );
    assert_eq!(player_entry(&view, 70101).appearance.inners, 1, "kept");
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently hidden"),
        "hidden reads hidden"
    );
    assert!(
        label_value(
            &nodes,
            "Error: The player wears inners under a hidden body, which breaks the hiding."
        ),
        "the inners finding's text on the section"
    );

    // Ctrl+I on a hidden player shows him: the show preset, "...shown".
    window.frame(vec![key(egui::Key::I, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        strip_of(&view, 70101),
        (1, true, 0, 0, 0),
        "the show preset"
    );
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(label_value(&nodes, "The default body is currently shown"));

    // Two operations, two undo entries, the player restored.
    assert_eq!(undo_entries(&mut window, &mut view), 2, "one entry each");

    // 70102's own boots ID 627: Hide keeps it, and he reads "FPC: body
    // shown" — his model hides him, the save cannot tell.
    view.selected = Some(Selection::Player(701, 1));
    let nodes = appearance_frame(&mut window, &mut view);
    click(
        &mut window,
        fpc_button(&nodes, "Hide default body"),
        &mut view,
    );
    assert_eq!(
        strip_of(&view, 70102),
        (2, false, 2, 627, 11),
        "the strip hidden, the own boots ID kept"
    );
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently shown"),
        "a model's own boots ID reads as shown"
    );
    click(
        &mut window,
        fpc_button(&nodes, "Show default body"),
        &mut view,
    );
    assert_eq!(
        strip_of(&view, 70102),
        (1, true, 0, 627, 0),
        "Show keeps the own ID too"
    );
    assert_eq!(undo_entries(&mut window, &mut view), 2, "one entry each");

    // The Team menu's Set FPC invisibility: the hide preset on every
    // filled slot of team 701, one undo step.
    let before: Vec<(usize, u32, u32, u32)> = rostered(&view, 701)
        .iter()
        .map(|&(slot, id)| {
            let player = player_entry(&view, id);
            (
                slot,
                id,
                player.appearance.boots_id,
                player.appearance.gloves_id,
            )
        })
        .collect();
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    click(
        &mut window,
        menu_item(&nodes, "Set FPC invisibility"),
        &mut view,
    );
    for &(slot, id, boots, gloves) in &before {
        let strip = strip_of(&view, id);
        assert_eq!(strip.0, 2, "slot {}'s sleeves", slot + 1);
        assert!(!strip.1, "slot {}'s tuck", slot + 1);
        assert_eq!(strip.2, 2, "slot {}'s socks", slot + 1);
        // A record's own ID stays, 0 or the hide ID takes the preset's.
        let expected_boots = if boots == 0 || boots == 55 { 55 } else { boots };
        assert_eq!(strip.3, expected_boots, "slot {}'s boots", slot + 1);
        let expected_gloves = if gloves == 0 || gloves == 11 {
            11
        } else {
            gloves
        };
        assert_eq!(strip.4, expected_gloves, "slot {}'s gloves", slot + 1);
    }
    assert_eq!(
        player_entry(&view, 70102).appearance.boots_id,
        627,
        "his own ID kept in the team run"
    );

    // Remove FPC invisibility is the same op the other way, run on the
    // hidden team: the show preset on every filled slot, the own IDs
    // still kept; one undo restores the hidden state.
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    click(
        &mut window,
        menu_item(&nodes, "Remove FPC invisibility"),
        &mut view,
    );
    for &(slot, id, boots, gloves) in &before {
        let strip = strip_of(&view, id);
        assert_eq!(strip.0, 1, "slot {}'s sleeves", slot + 1);
        assert!(strip.1, "slot {}'s tuck", slot + 1);
        assert_eq!(strip.2, 0, "slot {}'s socks", slot + 1);
        assert_eq!(strip.3, boots, "slot {}'s boots", slot + 1);
        assert_eq!(strip.4, gloves, "slot {}'s gloves", slot + 1);
    }
    // One undo (the Remove entry alone) hides them all again; the next is
    // the Set entry and restores the team's own state.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &(slot, id, boots, gloves) in &before {
        let strip = strip_of(&view, id);
        assert_eq!(strip.0, 2, "slot {}'s sleeves hidden again", slot + 1);
        assert_eq!(strip.2, 2, "slot {}'s socks hidden again", slot + 1);
        let expected_boots = if boots == 0 { 55 } else { boots };
        assert_eq!(strip.3, expected_boots, "slot {}'s boots", slot + 1);
        let expected_gloves = if gloves == 0 { 11 } else { gloves };
        assert_eq!(strip.4, expected_gloves, "slot {}'s gloves", slot + 1);
    }
    assert_eq!(undo_entries(&mut window, &mut view), 1, "the Set entry");
    for &(_, id, boots, gloves) in &before {
        let strip = strip_of(&view, id);
        assert_eq!(strip.3, boots, "restored");
        assert_eq!(strip.4, gloves, "restored");
    }
}

// Ctrl+I toggles both ways (the SE-GUI-11 test covers it hidden to
// shown): on a shown player it hides, again it shows, and the Player
// menu's Toggle FPC settings, enabled under the unlock, hides again.
// SE-GUI-11
#[test]
fn ctrl_i_toggles_the_fpc_visibility_both_ways_and_the_menu_item_too() {
    let path = fixture(PesVersion::Pes21, "fpc_ctrl_i");
    // 70101 dressed "body shown": boots and gloves 0.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    {
        let player = file.player_mut(70101).unwrap();
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
        player.appearance.sleeves = 1;
        player.appearance.untucked = true;
        player.appearance.socks = 0;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently shown"),
        "premise: shown"
    );

    // Ctrl+I on a shown player hides him: the hide preset lands.
    window.frame(vec![key(egui::Key::I, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently hidden"),
        "Ctrl+I hid him"
    );
    assert_eq!(strip_of(&view, 70101), (2, false, 2, 55, 11));

    // Ctrl+I again shows him: the show preset lands.
    window.frame(vec![key(egui::Key::I, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently shown"),
        "Ctrl+I showed him"
    );
    assert_eq!(strip_of(&view, 70101), (1, true, 0, 0, 0));

    // The Player menu's Toggle FPC settings, enabled under the unlock,
    // hides him again.
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    let item = menu_item(&nodes, "Toggle FPC settings");
    assert!(
        !item.is_disabled(),
        "the toggle is enabled under the unlock"
    );
    click(&mut window, item, &mut view);
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently hidden"),
        "the menu toggle hid him"
    );
    assert_eq!(strip_of(&view, 70101), (2, false, 2, 55, 11));
}

// The PES 15/16 half of the same scenario: the appearance block is a
// separate record there; Hide default body round-trips through Save and reopen.
// SE-GUI-11
#[test]
fn hide_body_round_trips_on_pes16() {
    let path = fixture(PesVersion::Pes16, "fpc_hide_16");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes16)).unwrap();
    {
        let player = file.player_mut(70101).unwrap();
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);

    let nodes = appearance_frame(&mut window, &mut view);
    click(
        &mut window,
        fpc_button(&nodes, "Hide default body"),
        &mut view,
    );
    assert_eq!(strip_of(&view, 70101), (2, false, 2, 55, 11));
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(label_value(&nodes, "The default body is currently hidden"));

    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let player = EditFile::load(&path)
        .unwrap()
        .player(70101)
        .unwrap()
        .clone();
    assert_eq!(
        (
            player.appearance.sleeves,
            player.appearance.untucked,
            player.appearance.socks,
            player.appearance.boots_id,
            player.appearance.gloves_id
        ),
        (2, false, 2, 55, 11),
        "the hide preset saved and reopened"
    );
}

// The FPC items' half of the lock: under it the section's Hide/Show default body,
// the Player menu's Toggle FPC settings and the Team menu's FPC items are
// disabled with the ownership line as hover; Ctrl+I is a no-op while Set
// all stats and Copy stats stay enabled.
// SE-GUI-22
#[test]
fn the_lock_disables_the_fpc_items_with_the_ownership_line() {
    let path = fixture(PesVersion::Pes21, "fpc_locked");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().appearance.boots_id = 0;
    file.player_mut(70101).unwrap().appearance.gloves_id = 0;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let before = player_entry(&view, 70101);

    // The section's two buttons disabled, each hover the ownership line.
    let nodes = appearance_frame(&mut window, &mut view);
    for label in ["Hide default body", "Show default body"] {
        let button = fpc_button(&nodes, label);
        assert!(button.is_disabled(), "{label} is disabled under the lock");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(button), "The team's export"),
            Some(
                "The team's export owns the aesthetics, so edit its settings.toml and recompile, then apply the patch."
                    .to_string(),
            ),
            "{label}'s hover is the ownership line"
        );
    }

    // The Player menu's Toggle FPC settings disabled the same way, the
    // gameplay items enabled.
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    let toggle = menu_item(&nodes, "Toggle FPC settings");
    assert!(
        toggle.is_disabled(),
        "the toggle is disabled under the lock"
    );
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(toggle), "The team's export"),
        Some(
            "The team's export owns the aesthetics, so edit its settings.toml and recompile, then apply the patch."
                .to_string(),
        ),
        "the toggle's hover is the ownership line"
    );
    for label in [
        "Set all stats",
        "Copy stats from",
        "Swap stats with",
        "Bump all stats",
    ] {
        assert!(
            !menu_item(&nodes, label).is_disabled(),
            "{label} stays enabled"
        );
    }

    // The Team menu's two FPC items disabled with the same hover.
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    for label in ["Set FPC invisibility", "Remove FPC invisibility"] {
        let item = menu_item(&nodes, label);
        assert!(item.is_disabled(), "{label} is disabled under the lock");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(item), "The team's export"),
            Some(
                "The team's export owns the aesthetics, so edit its settings.toml and recompile, then apply the patch."
                    .to_string(),
            ),
            "{label}'s hover is the ownership line"
        );
    }

    // Ctrl+I under the lock changes nothing.
    window.frame(vec![key(egui::Key::I, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(player_entry(&view, 70101), before, "Ctrl+I changed nothing");
    assert!(!view.session.as_ref().unwrap().is_dirty());
}

// The hide IDs are the common settings' (`view.hide_ids`): with IDs 60
// and 12, Hide default body writes them and the classifier reads them back; a
// player still on the convention's 55 reads "The default body is currently shown", the save
// itself never rewritten by the change.
// SE-GUI-31
#[test]
fn the_hide_ids_come_from_the_settings() {
    let path = fixture(PesVersion::Pes21, "fpc_hide_ids_setting");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    {
        let player = file.player_mut(70101).unwrap();
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
        player.appearance.sleeves = 1;
        player.appearance.untucked = true;
        player.appearance.socks = 0;
    }
    {
        let player = file.player_mut(70102).unwrap();
        player.appearance.boots_id = 55;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    view.hide_ids = fpc::HideIds::new(60, 12);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);

    // Hide default body on 70101 (IDs 0): boots 60 and gloves 12, "...hidden".
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently shown"),
        "premise: shown"
    );
    click(
        &mut window,
        fpc_button(&nodes, "Hide default body"),
        &mut view,
    );
    assert_eq!(strip_of(&view, 70101), (2, false, 2, 60, 12));
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(label_value(&nodes, "The default body is currently hidden"));

    // Show default body: both 0 again (the record holds the hide IDs).
    click(
        &mut window,
        fpc_button(&nodes, "Show default body"),
        &mut view,
    );
    assert_eq!(strip_of(&view, 70101), (1, true, 0, 0, 0));

    // 70102 still on the convention's boots 55 reads "The default body is currently shown":
    // the new hide ID is 60, and the save was never rewritten.
    view.selected = Some(Selection::Player(701, 1));
    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, "The default body is currently shown"),
        "the old convention's ID is no longer the hide ID"
    );
}

// The same setting carried through `Tool::view`: a `ToolContext` whose
// common settings hold 60 and 12, one frame of `tool.view`, and Hide
// body writes 60.
// SE-GUI-31
#[test]
fn the_tool_carries_the_common_hide_ids_into_the_view() {
    use studio_core::StudioTool;

    let path = fixture(PesVersion::Pes21, "fpc_hide_ids_context");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    {
        let player = file.player_mut(70101).unwrap();
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
    }
    file.save(&path).unwrap();
    let mut settings = studio_core::Settings::default();
    settings.common.fpc_hide_boots_id = 60;
    settings.common.fpc_hide_gloves_id = 12;
    // `Tool::view` refreshes the setting from the table each frame, so the
    // unlock's GIVEN is the table's, never a field set by hand.
    settings.set_tool(
        "save-editor",
        toml::from_str("aesthetics_from_exports = true").unwrap(),
    );
    let (ctx, _events_rx, _requests_rx) = tool_context(path.parent().unwrap(), settings);
    let mut tool = crate::Tool::new();
    tool.view.open(&path);
    tool.view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    open_appearance(&mut window, &mut tool.view);
    scroll_into_view(&mut window, &mut tool.view, |node| {
        node.role() == Role::Button && node.label() == Some("Hide default body")
    });
    window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert_eq!(
        tool.view.hide_ids,
        fpc::HideIds::new(60, 12),
        "the frame set the view's hide IDs from the settings"
    );

    // `click` draws through `show`, which lays the view out without what
    // `Tool::view` draws above it: the button is found on a `show` frame,
    // where the click lands (the hide IDs stay on the view).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut tool.view, &|| Vec::new()));
    click(
        &mut window,
        fpc_button(&nodes, "Hide default body"),
        &mut tool.view,
    );
    assert_eq!(
        strip_of(&tool.view, 70101).3,
        60,
        "Hide default body wrote 60"
    );
    assert_eq!(
        strip_of(&tool.view, 70101).4,
        12,
        "Hide default body wrote 12"
    );
}

// `view.hide_ids` an `Err` (a setting outside its band): the FPC line
// reads the error's text instead of "FPC: …", the section's buttons and the
// menus' FPC items are disabled with it as their hover, and Ctrl+I does
// nothing.
#[test]
fn an_out_of_band_hide_id_disables_the_fpc_paths_with_the_error() {
    let path = fixture(PesVersion::Pes21, "fpc_hide_ids_err");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().appearance.boots_id = 0;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    view.hide_ids = Err(fpc::HideIdsError::Boots(627));
    let error = fpc::HideIdsError::Boots(627).to_string();
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    let before = player_entry(&view, 70101);

    let nodes = appearance_frame(&mut window, &mut view);
    assert!(
        label_value(&nodes, &error),
        "the FPC line is the error's text"
    );
    assert!(
        !label_value(&nodes, "The default body is currently shown"),
        "the reading is not drawn"
    );
    for label in ["Hide default body", "Show default body"] {
        let button = fpc_button(&nodes, label);
        assert!(button.is_disabled(), "{label} is disabled");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(button), "fpc_hide_boots_id"),
            Some(error.clone()),
            "{label}'s hover adds a tooltip holding the error's text"
        );
    }

    // The Player menu's toggle disabled with the same hover; a gameplay
    // item stays enabled.
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    let toggle = menu_item(&nodes, "Toggle FPC settings");
    assert!(toggle.is_disabled());
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(toggle), "fpc_hide_boots_id"),
        Some(error.clone()),
        "the toggle's hover adds a tooltip holding the error's text"
    );
    assert!(
        !menu_item(&nodes, "Set all stats").is_disabled(),
        "Set all stats stays enabled"
    );

    // The Team menu's two items the same.
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    for label in ["Set FPC invisibility", "Remove FPC invisibility"] {
        let item = menu_item(&nodes, label);
        assert!(item.is_disabled(), "{label} is disabled");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(item), "fpc_hide_boots_id"),
            Some(error.clone()),
            "{label}'s hover adds a tooltip holding the error's text"
        );
    }

    // Ctrl+I changes nothing.
    window.frame(vec![key(egui::Key::I, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(player_entry(&view, 70101), before, "Ctrl+I changed nothing");
    assert!(!view.session.as_ref().unwrap().is_dirty());
}

// Every FPC path reads the same settings' hide IDs (`view.hide_ids` =
// 60 and 12): Ctrl+I hides and shows under them, the Player menu's
// Toggle FPC settings hides the same, and the Team menu's Set and
// Remove FPC invisibility write and clear them on every filled slot
// whose IDs were 0, a model's own kept.
// SE-GUI-31
#[test]
fn every_fpc_path_writes_the_settings_hide_ids() {
    let path = fixture(PesVersion::Pes21, "fpc_hide_ids_paths");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    {
        let player = file.player_mut(70101).unwrap();
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
        player.appearance.sleeves = 1;
        player.appearance.untucked = true;
        player.appearance.socks = 0;
    }
    {
        // A second slot whose IDs are 0, so the team run's write is not
        // the selected player's alone.
        let player = file.player_mut(70102).unwrap();
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
    }
    {
        // Hidden under the previous setting's 55 and 11: under 60 and 12
        // these are his own IDs, which the team's Set and Remove keep (the
        // fixture's other slots all hold 0).
        let player = file.player_mut(70103).unwrap();
        player.appearance.boots_id = 55;
        player.appearance.gloves_id = 11;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    view.hide_ids = fpc::HideIds::new(60, 12);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    assert_eq!(
        strip_of(&view, 70101),
        (1, true, 0, 0, 0),
        "premise: dressed, boots and gloves 0"
    );
    assert_eq!(
        (strip_of(&view, 70102).3, strip_of(&view, 70102).4),
        (0, 0),
        "premise: 70102's IDs are 0"
    );

    // Ctrl+I on a shown player hides him with the settings' IDs; again
    // it shows him.
    window.frame(vec![key(egui::Key::I, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        strip_of(&view, 70101),
        (2, false, 2, 60, 12),
        "Ctrl+I hides with the settings' IDs"
    );
    window.frame(vec![key(egui::Key::I, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        strip_of(&view, 70101),
        (1, true, 0, 0, 0),
        "Ctrl+I shows him again"
    );

    // The Player menu's Toggle FPC settings hides him the same way.
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    click(
        &mut window,
        menu_item(&nodes, "Toggle FPC settings"),
        &mut view,
    );
    assert_eq!(
        strip_of(&view, 70101),
        (2, false, 2, 60, 12),
        "the menu toggle hides with the settings' IDs"
    );

    // The Team menu's Set FPC invisibility: the settings' IDs on every
    // filled slot whose IDs were 0 or already the hide IDs, a model's
    // own kept.
    let before: Vec<(usize, u32, u32, u32)> = rostered(&view, 701)
        .iter()
        .map(|&(slot, id)| {
            let player = player_entry(&view, id);
            (
                slot,
                id,
                player.appearance.boots_id,
                player.appearance.gloves_id,
            )
        })
        .collect();
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    click(
        &mut window,
        menu_item(&nodes, "Set FPC invisibility"),
        &mut view,
    );
    for &(slot, id, boots, gloves) in &before {
        let strip = strip_of(&view, id);
        let expected_boots = if boots == 0 || boots == 60 { 60 } else { boots };
        let expected_gloves = if gloves == 0 || gloves == 12 {
            12
        } else {
            gloves
        };
        assert_eq!(
            (strip.3, strip.4),
            (expected_boots, expected_gloves),
            "slot {}'s IDs",
            slot + 1
        );
    }
    assert_eq!(
        (strip_of(&view, 70103).3, strip_of(&view, 70103).4),
        (55, 11),
        "the previous setting's IDs are kept as his own"
    );

    // Remove FPC invisibility zeroes the slots the Set wrote (0 or the
    // hide ID), a model's own kept again.
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    click(
        &mut window,
        menu_item(&nodes, "Remove FPC invisibility"),
        &mut view,
    );
    for &(slot, id, boots, gloves) in &before {
        let strip = strip_of(&view, id);
        let expected_boots = if boots == 0 || boots == 60 { 0 } else { boots };
        let expected_gloves = if gloves == 0 || gloves == 12 {
            0
        } else {
            gloves
        };
        assert_eq!(
            (strip.3, strip.4),
            (expected_boots, expected_gloves),
            "slot {}'s IDs back",
            slot + 1
        );
    }
    assert_eq!(
        (strip_of(&view, 70103).3, strip_of(&view, 70103).4),
        (55, 11),
        "Remove leaves the previous setting's IDs alone"
    );
}
