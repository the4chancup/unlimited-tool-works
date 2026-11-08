use super::*;

/// The checkbox labels under the `top` panel title down to `bottom`'s
/// (the whole column when `bottom` is `None`), in draw order: the first
/// column top to bottom, then the second.
fn checkboxes_in(nodes: &[accesskit::Node], top: &str, bottom: Option<&str>) -> Vec<String> {
    let top_y = label_bounds(nodes, top).y0;
    let bottom_y = bottom.map_or(f64::MAX, |label| label_bounds(nodes, label).y0);
    let mut cells: Vec<(f64, f64, String)> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::CheckBox
                && node
                    .bounds()
                    .is_some_and(|b| b.y0 > top_y + 1.0 && b.y0 < bottom_y - 1.0)
        })
        .filter_map(|node| {
            node.bounds()
                .and_then(|b| node.label().map(|label| (b.x0, b.y0, label.to_owned())))
        })
        .collect();
    cells.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("ordered bounds")
            .then(a.1.partial_cmp(&b.1).expect("ordered bounds"))
    });
    cells.into_iter().map(|(_, _, label)| label).collect()
}

/// `name`'s index in `labels::SKILLS`, the table the cards index into.
fn skill_index_of(name: &str) -> u8 {
    use pes_savefile::interchange::team_toml::labels::SKILLS;
    u8::try_from(
        SKILLS
            .iter()
            .position(|&label| label == name)
            .unwrap_or_else(|| panic!("{name} is no skill")),
    )
    .expect("a byte index")
}

/// `name`'s checkbox label as the cards' `card_name` draws it.
fn card_label(name: &str) -> String {
    pes_savefile::interchange::team_toml::labels::card_name(name)
}

/// `ids`' card labels in `SKILLS` order (the panels' draw order).
fn card_labels(ids: &[&str]) -> Vec<String> {
    let mut ids: Vec<u8> = ids.iter().map(|name| skill_index_of(name)).collect();
    ids.sort_unstable();
    ids.into_iter()
        .map(|index| {
            card_label(pes_savefile::interchange::team_toml::labels::SKILLS[usize::from(index)])
        })
        .collect()
}

/// `ruleset`'s `tricks` group's card labels in `SKILLS` order.
fn tricks_labels(ruleset: &aatf::Ruleset) -> Vec<String> {
    let mut ids: Vec<u8> = ruleset.card_groups["tricks"]
        .iter()
        .filter_map(|card| match card {
            aatf::Card::Skill(index) => Some(*index),
            aatf::Card::ComStyle(_) => None,
        })
        .collect();
    ids.sort_unstable();
    ids.into_iter()
        .map(|index| {
            card_label(pes_savefile::interchange::team_toml::labels::SKILLS[usize::from(index)])
        })
        .collect()
}

// The Skills panel lists the version's skills but the ruleset's tricks:
// those stand in their own panel between Skills and COM playing styles,
// in the game's order (Autumn 26's eleven), and the rest stay under
// Skills.
#[test]
fn the_tricks_panel_holds_the_rulesets_tricks() {
    let path = fixture(PesVersion::Pes21, "tricks_panel_autumn26");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let expected = tricks_labels(aatf::Rules::official().ruleset());
    assert_eq!(expected.len(), 11, "premise: Autumn 26's eleven tricks");
    assert_eq!(
        checkboxes_in(&nodes, "Tricks", Some("COM playing styles")),
        expected,
        "the Tricks panel's cards in the game's order"
    );
    let skills = checkboxes_in(&nodes, "Skills", Some("Tricks"));
    assert_eq!(
        skills.len(),
        41 - expected.len(),
        "the other skills stay under Skills"
    );
    for label in &expected {
        assert!(!skills.contains(label), "{label} is not under Skills");
    }
    assert!(
        skills.iter().any(|label| label == "Heading"),
        "a paid card the group does not name stays under Skills"
    );
}

// VGL 26's `tricks` group lists `double_touch` second and `malicia` last:
// the panel draws the twelve in the game's order anyway.
#[test]
fn the_tricks_panel_draws_the_cards_in_the_games_order() {
    let path = fixture(PesVersion::Pes21, "tricks_panel_vgl26");
    let ruleset = aatf::builtin("vgl26").expect("VGL26 is built in");
    let mut view = SaveView::new();
    view.open(&path);
    view.aatf_ruleset = "vgl26".to_owned();
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let expected = tricks_labels(ruleset.ruleset());
    assert_eq!(expected.len(), 12, "premise: VGL 26's twelve tricks");
    assert_eq!(
        checkboxes_in(&nodes, "Tricks", Some("COM playing styles")),
        expected,
        "the Tricks panel's cards in the game's order, not the file's"
    );
}

// A ruleset with no `tricks` group shows the skills its free-card rules
// name instead, those free only for a tier or the captain included
// (VTL 11's seventeen); a free COM style stays under COM playing styles.
#[test]
fn a_ruleset_without_tricks_shows_its_free_cards() {
    let path = fixture(PesVersion::Pes21, "free_cards_vtl11");
    let mut view = SaveView::new();
    view.open(&path);
    view.aatf_ruleset = "vtl11".to_owned();
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The 13 of "The free skills", the pass trio free to non-medals and
    // the captain's Captaincy.
    let expected = card_labels(&[
        "no_look_pass",
        "double_touch",
        "rabona",
        "acrobatic_finishing",
        "first_time_shot",
        "dipping_shots",
        "rising_shots",
        "heading",
        "malicia",
        "flip_flap",
        "marseille_turn",
        "scotch_move",
        "crossover_turn",
        "one_touch_pass",
        "weighted_pass",
        "pinpoint_crossing",
        "captaincy",
    ]);
    assert_eq!(expected.len(), 17, "premise: VTL 11's free skills");
    assert_eq!(
        checkboxes_in(&nodes, "Free cards", Some("COM playing styles")),
        expected,
        "the Free cards panel's skills in the game's order"
    );
    assert!(
        checkboxes_in(&nodes, "COM playing styles", None)
            .iter()
            .any(|label| label == "Long Ranger"),
        "a free COM style stays under COM playing styles"
    );
}

// A ruleset with no `tricks` group and no `free_cards` effects shows no
// aside panel at all: every skill stays under Skills (DBC7).
#[test]
fn a_ruleset_without_aside_cards_shows_no_aside_panel() {
    let path = fixture(PesVersion::Pes21, "no_aside_dbc7");
    let mut view = SaveView::new();
    view.open(&path);
    view.aatf_ruleset = "dbc7".to_owned();
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for title in ["Tricks", "Free cards"] {
        assert!(!label_value(&nodes, title), "no {title} title under DBC7");
    }
    assert_eq!(
        checkboxes_in(&nodes, "Skills", Some("COM playing styles")).len(),
        41,
        "every skill under Skills"
    );
}

// A trick's tick in the Tricks panel is the same one undoable edit as a
// skill's: the card's own `PlayerField::Skill` index.
#[test]
fn a_trick_tick_is_the_same_one_undoable_edit() {
    let path = fixture(PesVersion::Pes21, "trick_tick");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let held = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .skills
            .skills[usize::from(skill_index_of("double_touch"))]
    };
    let before = held(&view);

    // "Double Touch" is a trick: its checkbox sits under the Tricks
    // title, below the fold, so the tick goes through its AccessKit
    // action.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes: Vec<accesskit::Node> = pairs.iter().map(|(_, node)| node.clone()).collect();
    let tricks_y = label_bounds(&nodes, "Tricks").y0;
    let (card, cell) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::CheckBox && node.label() == Some("Double Touch"))
        .expect("the Double Touch checkbox");
    assert!(
        cell.bounds().expect("bounds").y0 > tricks_y,
        "premise: the trick's checkbox is under the Tricks title"
    );
    let card = *card;
    action_click(&mut window, &mut view, card);
    assert_ne!(held(&view), before, "the tick wrote the trick's own index");

    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(held(&view), before, "one undo restores it");
    assert!(!view.session.as_ref().unwrap().can_undo(), "one entry only");
}
