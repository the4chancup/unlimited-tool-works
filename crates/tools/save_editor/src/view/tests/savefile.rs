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
            "The team's export owns the aesthetics, so edit its settings.toml and recompile, then apply the patch."
                .to_string(),
        ),
        "its hover is the ownership line"
    );
    assert!(
        !node_labeled(&nodes, Role::Button, "Output rosters to TSV\u{2026}").is_disabled(),
        "the TSV item stays enabled: it writes nothing of the save's"
    );
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
/// one row, on the save's first and last team; the summary counts what
/// `check_teams` finds for the same teams; every team is listed under its
/// id and name with no suggestion. Narrowed to one team, it lists that
/// team alone; reversed, the line warns in the warning colour, nothing is
/// listed and Copy to clipboard is off.
// SE-GUI-41
#[test]
fn multi_team_aatf_checks_and_lists_every_team_of_the_range() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_range");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let ids = all_team_ids(&view);
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
    let pairs = multi_checked(&mut window, &mut view, &ids);
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
    assert_eq!(
        combos[0].value(),
        Some(names[0].as_str()),
        "the save's first team"
    );
    assert_eq!(
        combos[1].value(),
        Some(names[names.len() - 1].as_str()),
        "the save's last team"
    );

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

    // Reversed.
    pick_range_end(&mut window, &mut view, 1, &names[0]);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let lines = multi_lines(&pairs);
    let warning = "The first team comes after the last";
    assert!(lines.contains(&warning.to_owned()), "{lines:?}");
    assert_eq!(
        galley_color(text_shape(&shapes, warning)),
        window
            .ctx
            .style_of(window.ctx.theme())
            .visuals
            .warn_fg_color,
        "the warning colour"
    );
    assert!(
        !names.iter().skip(1).any(|name| lines.contains(name)),
        "nothing listed: {lines:?}"
    );
    assert!(multi_button(&pairs, "Copy to clipboard").is_disabled());
}

/// A team the check finds clean reads `All clean` under its line (714
/// /co/ of the PES 15 sample, made legal by the CLI's `official_legal.toml`;
/// the PES 17 sample holds no clean team under 4cc Autumn 26), and Copy
/// to clipboard says how many teams it copied.
// SE-GUI-41
#[test]
fn a_clean_team_reads_all_clean_and_copy_names_the_teams_copied() {
    let (mut view, mut window) = open_legal_714("multi_aatf_clean", Headless::new());
    open_multi_aatf(&mut window, &mut view);
    let ids = all_team_ids(&view);
    multi_checked(&mut window, &mut view, &ids);
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

    click(
        &mut window,
        multi_button(&pairs, "Copy to clipboard"),
        &mut view,
    );
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some("Copied the results of 5 teams.")
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
    let ids = all_team_ids(&view);
    let pairs = multi_checked(&mut window, &mut view, &ids);
    let lines = multi_lines(&pairs);
    let tier_row = |line: &String| line.starts_with("Tier count");
    assert!(
        lines.contains(&"734 /lit/".to_owned()) && lines.iter().any(tier_row),
        "premise: Autumn 26's list holds 734 and a tier count row: {lines:?}"
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
            .any(|line| line == "734 /lit/" || line.starts_with("Tier count")),
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
        nodes
            .iter()
            .filter_map(text)
            .any(|line| line == "734 /lit/"),
        "Spring 26's list is drawn"
    );
    let summary = format!("{} teams checked,", ids.len());
    assert!(
        nodes
            .iter()
            .filter_map(text)
            .any(|line| line.starts_with(&summary)),
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
