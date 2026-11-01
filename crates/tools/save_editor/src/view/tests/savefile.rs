use pes_savefile::model::KitSlot;

use super::*;

/// The Savefile menu's popup open for one frame: a click on the Savefile
/// button (skipped when a previous call left it open), then the frame's
/// nodes.
fn open_savefile_menu(window: &mut Headless, view: &mut SaveView) -> Vec<accesskit::Node> {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    if nodes.iter().any(|node| {
        node.role() == Role::Button && node.label() == Some("Output rosters to TSV\u{2026}")
    }) {
        return nodes;
    }
    click(window, menu_button(&nodes, "Savefile"), view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    nodes
}

// SE-GUI-27
#[test]
fn fix_database_repairs_the_save_behind_a_confirmation_and_one_undo() {
    let path = fixture(PesVersion::Pes17, "savefile_fix_database");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes17)).unwrap();
    // The three base-copy cases: the flag off with another player's id
    // (the id clears), the flag on with his own id (the flag clears), the
    // flag on with another player's id (a real base copy, untouched).
    {
        let player = file.player_mut(70101).unwrap();
        player.edit_flags.base_copy = Some(false);
        player.appearance.base_copy_id = 70102;
    }
    {
        let player = file.player_mut(70102).unwrap();
        player.edit_flags.base_copy = Some(true);
        player.appearance.base_copy_id = 70102;
    }
    {
        let player = file.player_mut(70103).unwrap();
        player.edit_flags.base_copy = Some(true);
        player.appearance.base_copy_id = 70104;
    }
    for id in [70101u32, 70102] {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.team_mut(701).unwrap().edit_flags.strip = Some(true);
    file.team_mut(701)
        .unwrap()
        .kit_slots
        .as_mut()
        .expect("PES 17 kit slots")[0] = KitSlot {
        number: 5,
        binding: 12345,
    };
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    // The menu item opens the confirmation; Cancel leaves the session clean.
    let nodes = open_savefile_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Fix database\u{2026}"),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(
            &nodes,
            "Repair base copies, clear every player's visual flags and the teams' strip \
             flag, and reset the kit slots to their defaults?"
        ),
        "the confirmation's question"
    );
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Cancel"),
        &mut view,
    );
    assert!(
        !view.session.as_ref().unwrap().is_dirty(),
        "Cancel changed nothing"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "Cancel pushed no undo entry"
    );

    // Fix database applies the whole repair.
    let nodes = open_savefile_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Fix database\u{2026}"),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Fix database"),
        &mut view,
    );
    assert_eq!(
        player_entry(&view, 70101).appearance.base_copy_id,
        0,
        "the flag-off stray copy id clears"
    );
    assert_eq!(
        player_entry(&view, 70102).edit_flags.base_copy,
        Some(false),
        "the baseless flag clears"
    );
    let third = player_entry(&view, 70103);
    assert_eq!(
        third.edit_flags.base_copy,
        Some(true),
        "a real base copy keeps its flag"
    );
    assert_eq!(third.appearance.base_copy_id, 70104, "and its copy id");
    for id in [70101u32, 70102] {
        let flags = &player_entry(&view, id).edit_flags;
        assert!(
            !(flags.face || flags.hair || flags.physique || flags.strip),
            "{id}'s visual flags cleared"
        );
    }
    let team = team_entry(&view, 701);
    assert_eq!(team.edit_flags.strip, Some(false), "the strip flag cleared");
    let slots = team.kit_slots.expect("PES 17 kit slots");
    assert_eq!(
        slots[0],
        KitSlot {
            number: 0,
            binding: 701 * 64
        }
    );
    assert_eq!(
        slots[1],
        KitSlot {
            number: 1,
            binding: 701 * 64
        }
    );
    assert_eq!(
        slots[2],
        KitSlot {
            number: 0x80,
            binding: 701 * 64
        }
    );
    for slot in &slots[3..] {
        assert_eq!(
            *slot,
            KitSlot {
                number: 0,
                binding: 0xFFFFC0
            }
        );
    }

    // One undo step restores every one of them.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(player_entry(&view, 70101).appearance.base_copy_id, 70102);
    assert_eq!(player_entry(&view, 70102).edit_flags.base_copy, Some(true));
    for id in [70101u32, 70102] {
        let flags = &player_entry(&view, id).edit_flags;
        assert!(
            flags.face && flags.hair && flags.physique && flags.strip,
            "{id}'s flags restored"
        );
    }
    let team = team_entry(&view, 701);
    assert_eq!(team.edit_flags.strip, Some(true), "the strip flag restored");
    assert_eq!(
        team.kit_slots.expect("kit slots")[0],
        KitSlot {
            number: 5,
            binding: 12345
        },
        "the kit slot restored"
    );

    // Redo, save, reopen: the file holds the repair.
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    assert_eq!(reopened.player(70101).unwrap().appearance.base_copy_id, 0);
    assert_eq!(
        reopened.player(70102).unwrap().edit_flags.base_copy,
        Some(false)
    );
    assert_eq!(
        reopened.player(70103).unwrap().edit_flags.base_copy,
        Some(true)
    );
    assert_eq!(
        reopened.player(70103).unwrap().appearance.base_copy_id,
        70104
    );
    assert_eq!(
        reopened.team(701).unwrap().edit_flags.strip,
        Some(false),
        "the saved strip flag"
    );
    assert_eq!(
        reopened.team(701).unwrap().kit_slots.expect("kit slots")[0],
        KitSlot {
            number: 0,
            binding: 701 * 64
        },
        "the saved kit slot"
    );
}

// The confirmation names only the repairs the open save's version
// carries: every clause derives from the records themselves, the visual
// flags being the one repair every version has.
// SE-GUI-27
#[test]
fn the_fix_database_question_names_only_what_the_save_has() {
    for (version, question) in [
        (PesVersion::Pes21, "Clear every player's visual flags?"),
        (
            PesVersion::Pes17,
            "Repair base copies, clear every player's visual flags and the teams' strip flag, \
             and reset the kit slots to their defaults?",
        ),
    ] {
        let path = fixture(version, &format!("fix_db_question_{}", version.number()));
        let mut view = SaveView::new();
        view.open(&path);
        view.aesthetics_from_exports = true;
        view.edit_aesthetics = true;
        let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
        let nodes = open_savefile_menu(&mut window, &mut view);
        click(
            &mut window,
            node_labeled(&nodes, Role::Button, "Fix database\u{2026}"),
            &mut view,
        );
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert!(
            label_value(&nodes, question),
            "{version}'s question is {question:?}"
        );
        click(
            &mut window,
            node_labeled(&nodes, Role::Button, "Cancel"),
            &mut view,
        );
    }
}

// The lock's Savefile half: under the lock Fix database is disabled with
// the ownership line as its hover, while the read-only TSV item stays
// enabled.
// SE-GUI-22
#[test]
fn the_locked_savefile_menu_disables_fix_database_only() {
    let path = fixture(PesVersion::Pes21, "savefile_menu_locked");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = open_savefile_menu(&mut window, &mut view);

    let item = node_labeled(&nodes, Role::Button, "Fix database\u{2026}");
    assert!(
        item.is_disabled(),
        "Fix database is disabled under the lock"
    );
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(item), "The team's export"),
        Some(
            "The team's export owns the aesthetics, so edit its settings.toml (the face, hair and iris: its ingame_face.toml) and recompile, then apply the patch."
                .to_string(),
        ),
        "its hover is the ownership line"
    );
    assert!(
        !node_labeled(&nodes, Role::Button, "Output rosters to TSV\u{2026}").is_disabled(),
        "the TSV item stays enabled: it writes nothing of the save's"
    );
}

// The open save is refused as the roster sheet's destination: the
// refusal names it, and the save's bytes stay.
#[test]
fn output_rosters_to_tsv_refuses_the_open_save() {
    let path = fixture(PesVersion::Pes21, "tsv_refuses_the_save");
    let before = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.rosters_dialog = Box::new({
        let path = path.clone();
        move |_| Some(path.clone())
    });
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    let nodes = open_savefile_menu(&mut window, &mut view);
    click(
        &mut window,
        menu_item(&nodes, "Output rosters to TSV\u{2026}"),
        &mut view,
    );
    assert!(
        std::fs::read(&path).unwrap() == before,
        "the save's bytes are unchanged"
    );
    // The refusal's modal spends its first frame sizing itself, with no
    // nodes; the last of the frames drawn here holds them.
    draw(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    let sentence = format!(
        "{}: that is the save being read, and a roster sheet written over it would lose it",
        path.display()
    );
    for text in ["Cannot write the roster sheet", sentence.as_str()] {
        assert!(label_value(&nodes, text), "the refusal shows {text:?}");
    }
}

/// The Multi-team AATF dialog's title under 4cc Autumn 26, the ruleset a
/// view starts on.
const MULTI_TITLE: &str = "Multi-team AATF (4cc Autumn 26)";

/// The Savefile menu's Multi-team AATF… clicked.
fn open_multi_aatf(window: &mut Headless, view: &mut SaveView) {
    let nodes = open_savefile_menu(window, view);
    click(window, menu_item(&nodes, "Multi-team AATF\u{2026}"), view);
}

/// Frames until the dialog's check of `ids` lands (bounded, so a worker
/// that never replies fails rather than hangs). Returns the landing
/// frame's node pairs.
fn multi_checked(
    window: &mut Headless,
    view: &mut SaveView,
    ids: &[u32],
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    frames_until(
        window,
        view,
        |view| {
            let check = &view.multi_aatf.check;
            check.pending.is_none() && check.result.as_ref().is_some_and(|(key, _)| key.ids == ids)
        },
        "the Multi-team AATF check never returned",
    )
}

/// The dialog's Label values and Button labels in top-to-bottom order
/// (the view's nodes behind its backdrop left out).
fn multi_lines(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<String> {
    let mut lines: Vec<(f64, f64, String)> = super::prompt::dialog_nodes(pairs, MULTI_TITLE)
        .into_iter()
        .filter_map(|node| {
            let text = match node.role() {
                Role::Label => node.value(),
                _ => node.label(),
            }?;
            let bounds = node.bounds()?;
            Some((bounds.y0, bounds.x0, text.to_owned()))
        })
        .collect();
    lines.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("drawn bounds")
            .then(a.1.partial_cmp(&b.1).expect("drawn bounds"))
    });
    lines.into_iter().map(|(_, _, text)| text).collect()
}

/// The dialog's two team combos, left to right.
fn range_combos(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<&accesskit::Node> {
    let mut combos: Vec<&accesskit::Node> = super::prompt::dialog_nodes(pairs, MULTI_TITLE)
        .into_iter()
        .filter(|node| node.role() == Role::ComboBox)
        .collect();
    combos.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().expect("drawn bounds").x0;
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    combos
}

/// `label` picked in the dialog's team combo `nth` from the left: the
/// combo clicked open, the popup's item picked through its AccessKit
/// action (a popup item can sit where a pointer click lands on nothing),
/// then a click on the title, which closes the popup the action leaves
/// open.
pub(super) fn pick_range_end(window: &mut Headless, view: &mut SaveView, nth: usize, label: &str) {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let combo = range_combos(&pairs)[nth];
    // The popup's items stand under the combo: the player list's team row
    // behind the dialog carries the same label.
    let (left, right) = {
        let bounds = combo.bounds().expect("drawn bounds");
        (bounds.x0, bounds.x1)
    };
    click(window, combo, view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let (item, _) = pairs
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button
                && node.label() == Some(label)
                && node
                    .bounds()
                    .is_some_and(|bounds| (left..right).contains(&bounds.x0))
        })
        .unwrap_or_else(|| panic!("no team {label:?} in the combo's popup"));
    action_click(window, view, *item);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let title = super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
        .into_iter()
        .find(|node| node.value() == Some(MULTI_TITLE))
        .expect("the title");
    click(window, title, view);
}

/// The dialog's button `label`.
fn multi_button<'a>(
    pairs: &'a [(accesskit::NodeId, accesskit::Node)],
    label: &str,
) -> &'a accesskit::Node {
    super::prompt::dialog_nodes(pairs, MULTI_TITLE)
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no {label:?} button"))
}

/// The Savefile menu's Multi-team AATF… on the PES 17 sample under 4cc
/// Autumn 26: the title names the ruleset; the range's two combos sit on
/// one row, both on the save's first team, 701, which the list holds
/// alone. Widened to the save's last team, the summary counts what
/// `check_teams` finds for the same teams; every team is listed under its
/// id and name with no suggestion. Narrowed to one team, it lists that
/// team alone.
// SE-GUI-41
#[test]
fn multi_team_aatf_checks_and_lists_every_team_of_the_range() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_range");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let ids = all_team_ids(&view);
    assert_eq!(ids[0], 701, "premise: 701 is the save's lowest team id");
    let first = format!("701 {}", display_name(&team_entry(&view, 701).name));
    let pairs = multi_checked(&mut window, &mut view, &[701]);
    let lines = multi_lines(&pairs);
    assert_eq!(lines[0], MULTI_TITLE);
    let combos = range_combos(&pairs);
    assert_eq!(combos.len(), 2, "two team combos");
    let middle = |node: &accesskit::Node| {
        let bounds = node.bounds().expect("drawn bounds");
        (bounds.y0 + bounds.y1) / 2.0
    };
    assert_eq!(
        middle(combos[0]),
        middle(combos[1]),
        "the combos share a row"
    );
    let ends: Vec<Option<&str>> = combos.iter().map(|combo| combo.value()).collect();
    assert_eq!(
        ends,
        [Some(first.as_str()), Some(first.as_str())],
        "both ends on the save's first team"
    );
    let summary = lines
        .iter()
        .find(|line| line.contains("checked,"))
        .unwrap_or_else(|| panic!("a summary in {lines:?}"));
    assert!(summary.starts_with("1 team checked, "), "{lines:?}");
    assert!(lines.contains(&first), "701 is listed: {lines:?}");
    assert!(
        !lines.iter().any(|line| line.starts_with("702 ")),
        "702 is not listed: {lines:?}"
    );

    // Widened to every team.
    let (names, expected_issues) = {
        let file = view.session.as_ref().expect("the save opened").file();
        let names: Vec<String> = ids
            .iter()
            .map(|&id| format!("{id} {}", display_name(&file.team(id).unwrap().name)))
            .collect();
        let choice = crate::session::rules::resolve("autumn26", &[]);
        let checks = crate::session::rules::check_teams(
            &choice.rules,
            &crate::session::rules::team_views(file, &ids),
        );
        let with_issues = checks
            .iter()
            .filter(|check| {
                check
                    .as_ref()
                    .expect("every sample team checks")
                    .violations
                    .iter()
                    .any(|violation| {
                        matches!(
                            violation.severity,
                            aatf::Severity::Error | aatf::Severity::Warning
                        )
                    })
            })
            .count();
        (names, with_issues)
    };
    pick_range_end(&mut window, &mut view, 1, &names[names.len() - 1]);
    let pairs = multi_checked(&mut window, &mut view, &ids);
    let lines = multi_lines(&pairs);
    let summary = format!("{} teams checked, {expected_issues} with issues", ids.len());
    assert!(lines.contains(&summary), "{summary:?} in {lines:?}");
    for name in &names {
        assert!(lines.contains(name), "{name:?} is listed");
    }
    assert!(
        !lines.iter().any(|line| line.starts_with("Unused")),
        "no suggestion is listed"
    );
    assert!(!multi_button(&pairs, "Copy to clipboard").is_disabled());

    // Narrowed to one team.
    pick_range_end(&mut window, &mut view, 0, "734 /lit/");
    pick_range_end(&mut window, &mut view, 1, "734 /lit/");
    let pairs = multi_checked(&mut window, &mut view, &[734]);
    let lines = multi_lines(&pairs);
    assert!(
        lines.contains(&"1 team checked, 1 with issues".to_owned()),
        "{lines:?}"
    );
    let listed: Vec<&String> = names.iter().filter(|name| lines.contains(name)).collect();
    assert_eq!(listed, ["734 /lit/"], "734 alone is listed");
}

/// The line above the list counts the systems the checked teams were read
/// under: Autumn 26 holds two height systems and no medal ones, and 706
/// reads under Red, 707 under Green, so the range 706 to 707 draws
/// `Height systems: Green 1, Red 1` between the summary and the teams.
// SE-GUI-51
#[test]
fn the_counts_line_lists_the_systems_the_range_was_read_under() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_counts");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let label =
        |view: &SaveView, id: u32| format!("{id} {}", display_name(&team_entry(view, id).name));
    let (first, last) = (label(&view, 706), label(&view, 707));
    pick_range_end(&mut window, &mut view, 0, &first);
    pick_range_end(&mut window, &mut view, 1, &last);
    let pairs = multi_checked(&mut window, &mut view, &[706, 707]);
    let lines = multi_lines(&pairs);
    let counts = "Height systems: Green 1, Red 1".to_owned();
    assert!(
        lines.contains(&counts),
        "the counts line is drawn: {lines:?}"
    );
    let at = lines.iter().position(|line| *line == counts).unwrap();
    let summary = lines
        .iter()
        .position(|line| line.contains("checked,"))
        .expect("the summary line");
    assert!(summary < at, "the counts sit below the summary: {lines:?}");
    // `rposition`: the first end's combo carries the same label, higher.
    let first = lines
        .iter()
        .rposition(|line| line.starts_with("706 "))
        .expect("the list's first team");
    assert!(at < first, "the counts sit above the list: {lines:?}");
}

/// The popup items of the dialog's team combo `nth` from the left, top to
/// bottom: the combo clicked open, its items read, then a click on the
/// title closes the popup.
fn range_items(window: &mut Headless, view: &mut SaveView, nth: usize) -> Vec<String> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let combo = range_combos(&pairs)[nth];
    let (left, right, below) = {
        let bounds = combo.bounds().expect("drawn bounds");
        (bounds.x0, bounds.x1, bounds.y1)
    };
    click(window, combo, view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    // The popup's items stand under the combo, as `pick_range_end` finds
    // them; its scroll area draws the items it shows.
    let mut items: Vec<(f64, String)> = pairs
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .filter_map(|(_, node)| {
            let (bounds, label) = (node.bounds()?, node.label()?);
            ((left..right).contains(&bounds.x0) && bounds.y0 >= below)
                .then(|| (bounds.y0, label.to_owned()))
        })
        .collect();
    items.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    let title = super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
        .into_iter()
        .find(|node| node.value() == Some(MULTI_TITLE))
        .expect("the title");
    click(window, title, view);
    items.into_iter().map(|(_, label)| label).collect()
}

/// The last end follows a first end picked past it, on the frame of the
/// pick, and lists the teams from the first end's on: 718 picked on the
/// first end of the PES 17 sample's dialog, opened on 701 to 701.
// SE-GUI-41
#[test]
fn the_last_end_follows_the_first_and_lists_the_teams_from_it() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_clamp");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The opening check lands before any bounds are read: its counts
    // line moves the widgets under the summary a row, so a click aimed
    // before it could land on another.
    multi_checked(&mut window, &mut view, &[701]);
    let label =
        |view: &SaveView, id: u32| format!("{id} {}", display_name(&team_entry(view, id).name));
    let (team_717, team_718) = (label(&view, 717), label(&view, 718));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        range_combos(&pairs)[1].value(),
        Some(label(&view, 701).as_str()),
        "premise: the last end opens on 701"
    );
    assert!(
        range_items(&mut window, &mut view, 1).contains(&team_717),
        "premise: the last end lists 717 while the first is on 701"
    );

    // `pick_range_end` by hand, so the pick's own frame is read.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = range_combos(&pairs)[0];
    let (left, right) = {
        let bounds = combo.bounds().expect("drawn bounds");
        (bounds.x0, bounds.x1)
    };
    click(&mut window, combo, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (item, _) = pairs
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button
                && node.label() == Some(team_718.as_str())
                && node
                    .bounds()
                    .is_some_and(|bounds| (left..right).contains(&bounds.x0))
        })
        .expect("718 in the first end's popup");
    let (_, pairs, _) = window.frame_ids(
        vec![egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: *item,
                data: None,
            },
        )],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    // The first combo drew its text before its popup took the pick; the
    // last, drawn after, already reads the pick.
    assert_eq!(
        range_combos(&pairs)[1].value(),
        Some(team_718.as_str()),
        "the last end follows the first on the pick's frame"
    );
    let title = super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
        .into_iter()
        .find(|node| node.value() == Some(MULTI_TITLE))
        .expect("the title");
    click(&mut window, title, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let ends: Vec<Option<&str>> = range_combos(&pairs)
        .into_iter()
        .map(accesskit::Node::value)
        .collect();
    assert_eq!(
        ends,
        [Some(team_718.as_str()), Some(team_718.as_str())],
        "both ends on 718"
    );

    let items = range_items(&mut window, &mut view, 1);
    assert_eq!(
        items.first(),
        Some(&team_718),
        "the last end's items start at 718: {items:?}"
    );
    assert!(!items.contains(&team_717), "717 is not offered: {items:?}");
}

/// A press on the dialog's title held while the opening check lands: the
/// result draws the counts line, and unless its auto id is kept when it
/// is absent, every widget after it takes the next id on the landing
/// frame, so the release clicks whichever widget took the pressed id
/// (Close). Released after the landing, the press leaves the dialog open.
#[test]
fn a_press_held_while_the_check_lands_leaves_the_dialog_open() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_held_click");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The opening check's reply is kept off the frames: the real run's
    // channel is taken and replaced with the test's, its reply kept
    // until the press is down.
    let (_, _, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let real = view
        .multi_aatf
        .check
        .pending
        .take()
        .expect("the opening check is in flight");
    let (sender, replies) = crossbeam_channel::unbounded();
    view.multi_aatf.check.pending = Some(worker::Pending {
        key: real.key.clone(),
        replies,
    });
    let reply = real
        .replies
        .recv_timeout(Duration::from_secs(120))
        .expect("the opening check never replied");

    // A frame while the press is still up: no result is listed.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.multi_aatf.check.result.is_none(),
        "premise: no result is listed yet"
    );
    let title = super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
        .into_iter()
        .find(|node| node.value() == Some(MULTI_TITLE))
        .expect("the title");
    let at = centre(title);

    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    sender
        .send(reply)
        .expect("the held channel takes the reply");
    let (_, _, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.multi_aatf
            .check
            .result
            .as_ref()
            .is_some_and(|(key, _)| key.ids == [701]),
        "premise: the reply landed while the press was held"
    );
    // The frame after the release's: a Close clicked on the release
    // frame closes the dialog on this one.
    let (_, _, _) = window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        pairs
            .iter()
            .any(|(_, node)| node.value() == Some(MULTI_TITLE)),
        "the dialog is still open"
    );
}

/// Thirty wheel notches over the first end, one a frame at 60 frames a
/// second, step it from 701 to 731, the last end following, and start no
/// check while they turn: the list keeps 701's result, dimmed to 0.55 and
/// marked checking. Once the input time passes 500 ms after the last
/// notch, one check starts, for 731 alone, and the list is drawn at full
/// opacity once it lands.
// SE-GUI-41
#[test]
fn a_range_turned_by_the_wheel_is_checked_once_it_has_rested() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_rest");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let ids = all_team_ids(&view);
    assert_eq!(
        ids[..=30],
        (701..=731).collect::<Vec<u32>>()[..],
        "premise: the save's first 31 teams are 701 to 731"
    );
    let label =
        |view: &SaveView, id: u32| format!("{id} {}", display_name(&team_entry(view, id).name));
    let first = label(&view, 701);
    multi_checked(&mut window, &mut view, &[701]);
    // Every other run of the view lands before the turn, so no reply lands
    // between two notches: under load the second notch once went
    // unstepped, and a reply landing mid-turn is the one input whose
    // timing the test does not set.
    workers_replied(&mut window, &mut view);
    // The colour the text `text` is painted in last, as the tessellator
    // picks it: the override, else the glyphs' own, else the fallback for
    // glyphs left to it. egui dims a painter's shapes by scaling these
    // colours. Painted last: the view behind the modal and the first
    // combo paint the same text before the list does.
    let ink = |shapes: &[egui::epaint::ClippedShape], text: &str| {
        shapes
            .iter()
            .rev()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text == text => Some(shape),
                _ => None,
            })
            .map(|shape| {
                let glyph = shape
                    .galley
                    .rows
                    .iter()
                    .flat_map(|row| &row.row.visuals.mesh.vertices)
                    .map(|vertex| vertex.color)
                    .find(|&color| color != egui::Color32::PLACEHOLDER);
                shape
                    .override_text_color
                    .or(glyph)
                    .unwrap_or(shape.fallback_color)
            })
    };
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(range_combos(&pairs)[0]);
    let full = ink(&shapes, &first).expect("701's line is painted");
    let time = |window: &Headless| window.ctx.input(|input| input.time);
    let notch_down = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    };
    let mut before = time(&window);
    for step in 1..=30u32 {
        let (_, pairs, shapes) = window.frame_ids(
            vec![egui::Event::PointerMoved(at), notch_down.clone()],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
        let now = time(&window);
        assert!(
            (now - before - 1.0 / 60.0).abs() < 1e-9,
            "premise: a frame is 1/60 s after the last, {before} then {now}"
        );
        before = now;
        // The first combo draws its text before the notch steps it; the
        // last, drawn after, reads the step.
        assert_eq!(
            range_combos(&pairs)[1].value(),
            Some(label(&view, 701 + step).as_str()),
            "notch {step} steps the first end, the last following"
        );
        let check = &view.multi_aatf.check;
        assert!(check.pending.is_none(), "notch {step} started a check");
        assert!(
            check
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.ids == [701]),
            "701's result is kept at notch {step}"
        );
        if step > 1 {
            let lines = multi_lines(&pairs);
            assert!(lines.contains(&first), "701's list is kept: {lines:?}");
            assert!(
                lines.contains(&"checking\u{2026}".to_owned()),
                "marked checking at notch {step}: {lines:?}"
            );
            assert_eq!(
                ink(&shapes, &first),
                Some(full.gamma_multiply(0.55)),
                "701's line dimmed to 0.55 at notch {step}"
            );
        }
    }
    let last_notch = before;

    // No more input: frames until the check lands, each counting the checks
    // started on it.
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut started = Vec::new();
    let mut shapes;
    loop {
        let had_pending = view.multi_aatf.check.pending.is_some();
        (_, _, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let check = &view.multi_aatf.check;
        if let Some(pending) = &check.pending
            && !had_pending
        {
            started.push((time(&window) - last_notch, pending.key.ids.clone()));
        }
        if check.pending.is_none()
            && check
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.ids == [731])
        {
            break;
        }
        assert!(Instant::now() < deadline, "731's check never returned");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(started.len(), 1, "one check started: {started:?}");
    let (after, checked) = &started[0];
    assert_eq!(checked, &[731], "the check is for the final range");
    assert!(
        *after >= 0.5 && *after < 0.5 + 3.0 / 60.0,
        "the check starts once 500 ms have passed since the last notch, not {after} s"
    );
    assert_eq!(
        ink(&shapes, &label(&view, 731)),
        Some(full),
        "731's line at full opacity, as 701's was"
    );
}

/// Multi-team AATF's range rests and is checked with no input to wake the
/// window: each frame of the rest asks egui for another within 500 ms, and
/// the frame the rest ends on asks for the next at once, which starts the
/// check. Asked for none, the list stays dimmed until the pointer moves.
// SE-GUI-41
#[test]
fn a_resting_range_asks_for_the_frames_its_check_needs() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_repaint");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    assert_eq!(
        all_team_ids(&view)[..2],
        [701, 702],
        "premise: the save's first two teams are 701 and 702"
    );
    multi_checked(&mut window, &mut view, &[701]);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let notch_down = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    };
    let events = vec![
        egui::Event::PointerMoved(centre(range_combos(&pairs)[0])),
        notch_down,
    ];
    let (_, pairs, _) = window.frame_ids(events, |ui| show(ui, &mut view, &|| Vec::new()));
    let label = format!("702 {}", display_name(&team_entry(&view, 702).name));
    assert_eq!(
        range_combos(&pairs)[1].value(),
        Some(label.as_str()),
        "premise: the notch picked 702"
    );

    // No more input: each frame's request, until the frame that starts
    // 702's check (30 frames of 1/60 s make the rest).
    let started = |view: &SaveView| {
        let check = &view.multi_aatf.check;
        check.pending.is_some()
            || check
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.ids == [702])
    };
    assert!(!started(&view), "premise: the notch started no check");
    let mut delays = Vec::new();
    while !started(&view) {
        assert!(delays.len() < 90, "702's check never started");
        window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        delays.push(window.repaint_delay);
    }
    let [resting @ .., promoted, _started] = delays.as_slice() else {
        panic!("the check started on the first frame after the notch: {delays:?}");
    };
    assert!(!resting.is_empty(), "premise: the range rested a frame");
    for (frame, delay) in resting.iter().enumerate() {
        assert!(
            *delay <= Duration::from_millis(500),
            "resting frame {frame} asks for the next within the rest, not {delay:?}"
        );
    }
    assert_eq!(
        *promoted,
        Duration::ZERO,
        "the frame the rest ends on asks for the next at once"
    );
}

/// The smallest painted rectangle of `fill` holding every one of `points`: a
/// frame as drawn, border included. A frame with a shadow paints as a
/// `Shape::Vec` of the shadow and the rectangle, so nested shapes count too.
fn painted_frame(
    shapes: &[egui::epaint::ClippedShape],
    fill: egui::Color32,
    points: &[egui::Pos2],
) -> egui::Rect {
    let mut rects = Vec::new();
    let mut stack: Vec<&egui::Shape> = shapes.iter().map(|clipped| &clipped.shape).collect();
    while let Some(shape) = stack.pop() {
        match shape {
            egui::Shape::Rect(rect) => rects.push(rect),
            egui::Shape::Vec(inner) => stack.extend(inner),
            _ => {}
        }
    }
    rects
        .iter()
        .filter(|rect| rect.fill == fill && points.iter().all(|&p| rect.rect.contains(p)))
        .map(|rect| rect.rect)
        .min_by(|a, b| a.area().total_cmp(&b.area()))
        .unwrap_or_else(|| panic!("no frame of {fill:?} around {points:?}"))
}

/// Multi-team AATF's dialog is 900 points wide, its list 874 of them (the
/// modal's padding and border around it), so a height row's sentence and
/// its players' lines have room.
#[test]
fn multi_team_aatf_is_900_points_wide() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The range opens on the save's first team alone.
    let ids = all_team_ids(&view);
    multi_checked(&mut window, &mut view, &ids[..1]);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The modal's frame and the list's, as painted: the smallest rectangle of
    // the window's fill holding the title and the range's first combo (a
    // panel of the view behind the modal shares the fill), then the smallest
    // of the extreme background under them.
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let title = centre(
        super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
            .into_iter()
            .find(|node| node.value() == Some(MULTI_TITLE))
            .expect("the title"),
    );
    let first_combo = centre(range_combos(&pairs)[0]);
    let painted = |fill, points: &[egui::Pos2]| painted_frame(&shapes, fill, points);
    let dialog = painted(visuals.window_fill(), &[title, first_combo]);
    assert!(
        (dialog.width() - 900.0).abs() <= 0.5,
        "the dialog's width: {}",
        dialog.width()
    );
    let list = painted(
        visuals.extreme_bg_color,
        &[egui::pos2(dialog.center().x, first_combo.y + 80.0)],
    );
    assert!(
        (list.width() - 874.0).abs() <= 0.5,
        "the list's outer width: {}",
        list.width()
    );
}

/// Multi-team AATF's list is 360 points tall, border included, when its
/// rows overflow it (the PES 17 sample's every team): it scrolls rather
/// than growing or shrinking.
#[test]
fn multi_team_aatfs_overflowing_list_is_360_points_tall() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_height");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let ids = all_team_ids(&view);
    let (first, last) = {
        let file = view.session.as_ref().expect("the save opened").file();
        let name = |id: u32| format!("{id} {}", display_name(&file.team(id).unwrap().name));
        (name(ids[0]), name(ids[ids.len() - 1]))
    };
    multi_checked(&mut window, &mut view, &ids[..1]);
    pick_range_end(&mut window, &mut view, 1, &last);
    multi_checked(&mut window, &mut view, &ids);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let dialog = super::prompt::dialog_nodes(&pairs, MULTI_TITLE);
    let first_team = centre(
        dialog
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(first.as_str()))
            .expect("the first team's line"),
    );
    let scroll_bar = centre(
        dialog
            .iter()
            .find(|node| node.role() == Role::ScrollBar)
            .expect("premise: the rows overflow the list, so it draws a scroll bar"),
    );
    // The list's frame as painted: the extreme background holding both the
    // first team's line and the scroll bar (the player list's well behind
    // the modal holds the line alone).
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let list = painted_frame(&shapes, visuals.extreme_bg_color, &[first_team, scroll_bar]);
    assert!(
        (list.height() - 360.0).abs() <= 1.0,
        "the list's outer height: {}",
        list.height()
    );
}

/// Multi-team AATF's range is kept while the save stays open: narrowed to
/// 734 alone, the dialog closed and opened again reads 734 at both ends.
#[test]
fn a_reopened_multi_team_aatf_keeps_its_range() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_reopen");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    pick_range_end(&mut window, &mut view, 0, "734 /lit/");
    pick_range_end(&mut window, &mut view, 1, "734 /lit/");
    let pairs = multi_checked(&mut window, &mut view, &[734]);

    click(&mut window, multi_button(&pairs, "Close"), &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !pairs
            .iter()
            .any(|(_, node)| node.value() == Some(MULTI_TITLE)),
        "premise: the dialog closed"
    );
    open_multi_aatf(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let ends: Vec<Option<&str>> = range_combos(&pairs)
        .into_iter()
        .map(accesskit::Node::value)
        .collect();
    assert_eq!(
        ends,
        [Some("734 /lit/"), Some("734 /lit/")],
        "the range is kept"
    );
}

/// A team the check finds clean reads `All clean` under its line (714
/// /co/ of the PES 15 sample, made legal by the CLI's `official_legal.toml`;
/// the PES 17 sample holds no clean team under 4cc Autumn 26), and Copy
/// to clipboard says how many teams it copied.
// SE-GUI-41
// SE-GUI-51
#[test]
fn a_clean_team_reads_all_clean_and_copy_names_the_teams_copied() {
    let (mut view, mut window) = open_legal_714("multi_aatf_clean", Headless::new());
    open_multi_aatf(&mut window, &mut view);
    // The range opens on the save's first team alone.
    let ids = all_team_ids(&view);
    multi_checked(&mut window, &mut view, &ids[..1]);
    pick_range_end(&mut window, &mut view, 0, "714 /co/");
    pick_range_end(&mut window, &mut view, 1, "718 /f/");
    let pairs = multi_checked(&mut window, &mut view, &[714, 715, 716, 717, 718]);
    let lines = multi_lines(&pairs);
    let at = lines
        .iter()
        .position(|line| line == "714 /co/")
        .expect("714 is listed");
    assert_eq!(lines[at + 1], "All clean", "{lines:?}");
    assert_eq!(
        lines.iter().filter(|line| *line == "All clean").count(),
        1,
        "the other teams have rows"
    );

    let text = copied(&mut window, &mut view, &pairs).expect("the click copies the list");
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some("Copied the results of 5 teams.")
    );
    let copied_lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        copied_lines[0], "Height systems: Green 2, Red 3",
        "the first line counts the systems the range read under"
    );
    assert!(
        copied_lines[1].starts_with("4cc Autumn 26, teams 714 to 718: "),
        "the second line names the ruleset and the range: {text}"
    );
    let teams: Vec<&str> = copied_lines
        .iter()
        .filter_map(|line| line.strip_prefix('>'))
        .collect();
    let expected: Vec<String> = (714..=718)
        .map(|id| format!("{id} {}", display_name(&team_entry(&view, id).name)))
        .collect();
    assert_eq!(teams, expected, "every team of the range after a `>`");
    let at = copied_lines
        .iter()
        .position(|line| *line == ">714 /co/")
        .expect("714 in the copied text");
    assert_eq!(copied_lines[at + 1], "  All clean", "{text}");
}

/// Copy to clipboard clicked (press on one frame, release on the next):
/// the text the release frame's `CopyText` command carries, `None` when it
/// copies nothing.
fn copied(
    window: &mut Headless,
    view: &mut SaveView,
    pairs: &[(accesskit::NodeId, accesskit::Node)],
) -> Option<String> {
    let at = centre(multi_button(pairs, "Copy to clipboard"));
    window.frame(pointer_at(at, true), |ui| show(ui, view, &|| Vec::new()));
    let (commands, _, _) = window.frame(pointer_at(at, false), |ui| {
        show(ui, view, &|| Vec::new());
        ui.ctx().output(|output| output.commands.clone())
    });
    commands.into_iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text),
        _ => None,
    })
}

/// A range where one team's check fails (743 under THROW19, whose custom
/// check throws on 743's captain) names that team with its error and lists
/// the other team's rows, in the drawn list and in the copied text.
#[test]
fn a_failed_teams_check_is_named_with_its_error_and_the_others_listed() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_failed");
    let mut view = SaveView::new();
    view.open(&path);
    pick_imported(&mut view, "throw19.4ccrules", &throw19());
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The range opens on the save's first team alone.
    let ids = all_team_ids(&view);
    multi_checked(&mut window, &mut view, &ids[..1]);
    let label =
        |view: &SaveView, id: u32| format!("{id} {}", display_name(&team_entry(view, id).name));
    let (failed, other) = (label(&view, 743), label(&view, 744));
    let other_rows = {
        let file = view.session.as_ref().expect("the save opened").file();
        let rules = aatf::Rules::load(&throw19()).expect("THROW19 loads");
        let checks = crate::session::rules::check_teams(
            &rules,
            &crate::session::rules::team_views(file, &[743, 744]),
        );
        assert!(checks[0].is_err(), "premise: 743's check fails");
        checks[1]
            .as_ref()
            .expect("premise: 744's check runs")
            .violations
            .iter()
            .filter(|violation| violation.severity != aatf::Severity::Suggestion)
            .count()
    };
    assert!(other_rows > 0, "premise: 744 has rows");
    pick_range_end(&mut window, &mut view, 0, &failed);
    pick_range_end(&mut window, &mut view, 1, &other);
    let pairs = multi_checked(&mut window, &mut view, &[743, 744]);

    let lines = multi_lines(&pairs);
    let failed_line = format!("The AATF check failed for team {failed}: ");
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with(&failed_line) && line.contains("boom")),
        "743 named with its error: {lines:?}"
    );
    assert!(
        lines.contains(&"2 teams checked, 1 with issues".to_owned()),
        "{lines:?}"
    );
    let at = lines
        .iter()
        .position(|line| *line == other)
        .expect("744 is listed");
    // The list's group headers: a 744xx player's head or the team's own.
    let group = &lines[at + 1];
    assert!(
        group.starts_with("Player 744") || group == "Team-wide issues",
        "a group header of 744's rows under its line: {lines:?}"
    );

    let text = copied(&mut window, &mut view, &pairs).expect("the click copies the list");
    let copied_lines: Vec<&str> = text.lines().collect();
    let at = copied_lines
        .iter()
        .position(|line| *line == format!(">{failed}"))
        .expect("743 in the copied text");
    assert!(
        copied_lines[at + 1].starts_with("  The AATF check failed: ")
            && copied_lines[at + 1].contains("boom"),
        "743's error under its line: {text}"
    );
    let at = copied_lines
        .iter()
        .position(|line| *line == format!(">{other}"))
        .expect("744 in the copied text");
    assert_eq!(
        copied_lines[at + 1..]
            .iter()
            .filter(|line| line.starts_with("    "))
            .count(),
        other_rows,
        "744's rows: {text}"
    );
}

/// A list checked under 4cc Autumn 26 is not drawn once 4cc Spring 26 is
/// picked, and Copy to clipboard is off: its first line would name Spring
/// 26 over Autumn 26's findings. The dialog reads as before its first
/// result until the recheck returns, then lists Spring 26's. The recheck
/// is held unanswered by a pending check whose sender the test keeps: the
/// dialog starts one on any frame, and its worker can reply between a
/// frame's two passes.
// SE-GUI-41
#[test]
fn a_list_checked_under_another_ruleset_is_not_drawn_nor_copied() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_other_ruleset");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The range opens on the save's first team alone.
    let ids = all_team_ids(&view)[..1].to_vec();
    let team = format!(
        "{} {}",
        ids[0],
        display_name(&team_entry(&view, ids[0]).name)
    );
    let pairs = multi_checked(&mut window, &mut view, &ids);
    let lines = multi_lines(&pairs);
    let tier_row = |line: &String| line.starts_with("Tier count");
    assert!(
        lines.contains(&team) && lines.iter().any(tier_row),
        "premise: Autumn 26's list holds {team} and a tier count row: {lines:?}"
    );

    view.aatf_ruleset = "spring26".to_owned();
    let (sender, replies) = crossbeam_channel::unbounded();
    view.multi_aatf.check.pending = Some(aatf_check::PendingCheck {
        key: aatf_check::CheckKey {
            revision: view.session.as_ref().expect("the save opened").revision(),
            ids: ids.clone(),
            ruleset: "spring26".to_owned(),
            suggestions: false,
        },
        replies,
    });
    let title = "Multi-team AATF (4cc Spring 26)";
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.multi_aatf
            .check
            .result
            .as_ref()
            .is_some_and(|(key, _)| key.ruleset == "autumn26"),
        "premise: Autumn 26's list held"
    );
    let nodes = super::prompt::dialog_nodes(&pairs, title);
    fn text<'a>(node: &&'a accesskit::Node) -> Option<&'a str> {
        match node.role() {
            Role::Label => node.value(),
            _ => node.label(),
        }
    }
    assert!(
        !nodes
            .iter()
            .filter_map(text)
            .any(|line| line == team || line.starts_with("Tier count")),
        "the held list's rows are not drawn"
    );
    assert!(
        !nodes
            .iter()
            .filter_map(text)
            .any(|line| line.contains("checked,")),
        "no summary before a result"
    );
    let copy = nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Copy to clipboard"))
        .expect("the Copy to clipboard button");
    assert!(copy.is_disabled(), "Copy to clipboard is off");

    // The held check's worker ends: the next frame starts Spring 26's.
    drop(sender);
    let pairs = frames_until(
        &mut window,
        &mut view,
        |view| {
            let check = &view.multi_aatf.check;
            check.pending.is_none()
                && check
                    .result
                    .as_ref()
                    .is_some_and(|(key, _)| key.ruleset == "spring26")
        },
        "Spring 26's check never returned",
    );
    let nodes = super::prompt::dialog_nodes(&pairs, title);
    assert!(
        nodes.iter().filter_map(text).any(|line| line == team),
        "Spring 26's list is drawn"
    );
    let summary = "1 team checked,";
    assert!(
        nodes
            .iter()
            .filter_map(text)
            .any(|line| line.starts_with(summary)),
        "Spring 26's summary"
    );
    let copy = nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Copy to clipboard"))
        .expect("the Copy to clipboard button");
    assert!(!copy.is_disabled(), "Copy to clipboard is on");
}

/// The open save's team ids, in id order.
fn all_team_ids(view: &SaveView) -> Vec<u32> {
    let file = view.session.as_ref().expect("the save opened").file();
    let mut ids: Vec<u32> = file.teams().iter().map(|team| team.id).collect();
    ids.sort_unstable();
    ids
}
