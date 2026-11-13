use super::*;

/// The bounds of the Skills column's header (`ui.strong("Skills")`):
/// what the Cards used row's tests scope to.
fn skills_header(nodes: &[accesskit::Node]) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("Skills"))
        .and_then(accesskit::Node::bounds)
        .expect("the Skills heading")
}

/// Whether the Skills column draws a Label with `value` (the column's x
/// at the header's or right of it).
fn in_skills_column(nodes: &[accesskit::Node], value: &str) -> bool {
    let column_x0 = skills_header(nodes).x0;
    nodes.iter().any(|node| {
        node.role() == Role::Label
            && node.value() == Some(value)
            && node.bounds().is_some_and(|b| b.x0 >= column_x0 - 1.0)
    })
}

/// The badge texts a frame's Cards used row drew (`"Regular: 3 of 7"`
/// or the `"No tier matches"` stand-in), in left-to-right order: the
/// Labels at the heading's height or below it of the badge's shapes
/// (labels of another shape must not pass). None while another player's
/// or ruleset's run is up: the row draws nothing then.
fn cards_used_badges(nodes: &[accesskit::Node]) -> Vec<String> {
    let Some(heading_y) = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Label && node.value().is_some_and(|v| v.starts_with("Cards used"))
        })
        .and_then(accesskit::Node::bounds)
        .map(|bounds| bounds.y0)
    else {
        return Vec::new();
    };
    let mut badges: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Label
                && node
                    .bounds()
                    .is_some_and(|bounds| bounds.y0 >= heading_y - 1.0)
                && node.value().is_some_and(|value| {
                    // The badge's text is "tier: paid of limit" or the
                    // stand-in; the "Free cards" panel's own title is not.
                    value == "No tier matches"
                        || value.rsplit_once(": ").is_some_and(|(_, count)| {
                            count.split_once(" of ").is_some_and(|(paid, limit)| {
                                paid.parse::<i32>().is_ok() && limit.parse::<u8>().is_ok()
                            })
                        })
                })
        })
        .collect();
    badges.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().map(|b| b.x0).unwrap_or(0.0);
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    badges
        .iter()
        .filter_map(|node| node.value().map(str::to_owned))
        .collect()
}

/// The `BadgeKey` a frame of `view` asks the badge for: the session's
/// revision, the selected player and the active ruleset.
fn current_badge_key(view: &SaveView) -> BadgeKey {
    let Some(Selection::Player(team, slot)) = view.selected else {
        panic!("a player is selected")
    };
    BadgeKey {
        revision: view.session.as_ref().expect("an open save").revision(),
        team,
        slot,
        ruleset: view
            .rules
            .as_ref()
            .expect("a frame resolved the ruleset")
            .key
            .clone(),
    }
}

/// Holds a Cards used run in flight on `view`'s current badge key: no
/// reply reaches a frame until the test sends one on the returned sender
/// or drops it (a real run can reply between one frame's two passes, so
/// it cannot be waited on where a test needs the in-flight frame).
fn hold_badge_run(view: &mut SaveView) -> crossbeam_channel::Sender<(BadgeKey, OwnBadge)> {
    let (sender, replies) = crossbeam_channel::unbounded();
    view.cards.pending = Some(worker::Pending {
        key: current_badge_key(view),
        replies,
    });
    sender
}

/// Frames until the badge worker's answer for the current key lands
/// (bounded, so a worker that never replies fails rather than hangs);
/// returns the landing frame's nodes, whose header row draws it.
fn badge_landed(window: &mut Headless, view: &mut SaveView) -> Vec<accesskit::Node> {
    frames_until(
        window,
        view,
        |view| {
            let key = current_badge_key(view);
            view.cards.pending.is_none()
                && view
                    .cards
                    .result
                    .as_ref()
                    .is_some_and(|(held, _)| *held == key)
        },
        "the badge worker never answered",
    )
    .into_iter()
    .map(|(_, node)| node)
    .collect()
}

// The same rule for a number control's in-progress edit: typing into one
// player's Speed and selecting another player commits nothing to him.
#[test]
fn a_speed_edit_does_not_reach_the_next_players_control() {
    let path = fixture(PesVersion::Pes17, "cross_player_speed");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let speed_of = |view: &SaveView, id: u32| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(id)
            .unwrap()
            .stats
            .speed
    };

    let file = EditFile::load(&path).unwrap();
    let a_id = file.team(702).unwrap().roster[0].player_id;
    let b_id = file.team(702).unwrap().roster[1].player_id;
    view.selected = Some(Selection::Player(702, 0));
    // The Speed row may sit below the detail pane's fold: scroll it inside
    // the clip, where it takes a click, before aiming the pointer.
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Speed")
    });
    let speed = right_of_label(&nodes, "Speed", Role::SpinButton)
        .first()
        .copied()
        .expect("the Speed control");
    click(&mut window, speed, &mut view);
    window.frame(vec![egui::Event::Text("55".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let before_b = speed_of(&view, b_id);

    // Select 70202 (roster slot 1) in the same sequence and draw his view:
    // his speed is untouched — and a keystroke reaches no control of his
    // (the in-progress edit stays 70201's own).
    view.selected = Some(Selection::Player(702, 1));
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    window.frame(vec![egui::Event::Text("9".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        speed_of(&view, b_id),
        before_b,
        "70202's Speed is unchanged"
    );
    assert_eq!(
        speed_of(&view, a_id),
        55,
        "the typed value landed on 70201 alone"
    );
}

// SE-GUI-03
#[test]
fn an_ability_edit_marks_the_row_until_ctrl_s_writes_the_save() {
    let path = fixture(PesVersion::Pes21, "ability_edit_save");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    // 70101's Speed is 77: focus the control and type a new one. The
    // Speed row may sit below the detail pane's fold: scroll it inside the
    // clip, where it takes a click, before aiming the pointer.
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Speed")
    });
    let speed = right_of_label(&nodes, "Speed", Role::SpinButton)
        .first()
        .expect("the Speed control")
        .bounds()
        .expect("bounds");
    assert_eq!(
        right_of_label(&nodes, "Speed", Role::SpinButton)[0].value(),
        Some("77"),
        "premise: Speed is 77"
    );
    let at = egui::pos2(
        ((speed.x0 + speed.x1) / 2.0) as f32,
        ((speed.y0 + speed.y1) / 2.0) as f32,
    );
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![egui::Event::Text("55".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    row(&nodes, "01 CB PLACEHOLDER ●");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the file is unchanged until Save"
    );

    // Ctrl+S writes it; the dot goes and the reopened file holds the value.
    let bak = path.with_file_name("EDIT00000000.bak");
    assert!(!bak.exists(), "no .bak before the first save");
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    row(&nodes, "01 CB PLACEHOLDER");
    assert_eq!(
        std::fs::read(&bak).unwrap(),
        original,
        "the .bak is the file as opened"
    );
    let reopened = Session::open(&path, Recorder::default()).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().stats.speed,
        55,
        "the reopened file holds the speed"
    );
}

// SE-GUI-06
#[test]
fn version_gated_controls_are_disabled_in_place_with_their_first_version() {
    // The stronger hand (the Profile block's gated control; the Cutscenes
    // pair's half of the scenario lives on the Appearance section's test):
    // the same player on PES 17 (no hand stored) and PES 21 (stored), the
    // Left | Right toggle in the same span, disabled with the `PES NN+`
    // tooltip on 17, Right held as on every save without the field.
    let bounds_of = |version: PesVersion, test: &str| {
        let path = fixture(version, test);
        let mut view = SaveView::new();
        view.open(&path);
        let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let sides = right_of_label(&nodes, "Stronger hand", Role::Button);
        assert_eq!(sides.len(), 2, "the hand is two choices");
        let span = |node: &accesskit::Node| node.bounds().expect("bounds");
        let hand = (
            span(sides[0]).x0,
            span(sides[1]).x1,
            span(sides[0]).y0,
            span(sides[1]).y1,
        );
        (view, window, nodes, hand)
    };
    let (mut view17, mut window17, nodes17, hand17) = bounds_of(PesVersion::Pes17, "gated_17");
    let (_view21, _window21, nodes21, hand21) = bounds_of(PesVersion::Pes21, "gated_21");

    let sides17 = right_of_label(&nodes17, "Stronger hand", Role::Button);
    assert!(
        sides17.iter().all(|side| side.is_disabled()),
        "the missing hand's sides are disabled"
    );
    assert_eq!(
        sides17[1].toggled(),
        Some(accesskit::Toggled::True),
        "the missing hand shows Right held"
    );
    assert_eq!(
        tooltip_after_hover(&mut window17, &mut view17, centre(sides17[1])),
        Some("PES 20+".to_string()),
        "the hand's tooltip"
    );

    // Enabled on PES 21, the toggle in the same span as its PES 17 stand-in.
    assert!(
        !control_right_of(&nodes21, "Stronger hand").is_disabled(),
        "Stronger hand is enabled on PES 21"
    );
    assert_eq!(hand17, hand21, "the Stronger hand control's span");
}

// A stat the version lacks shows `-`, disabled, its tooltip naming the first
// version that has it; the stats it stores draw enabled.
#[test]
fn a_stat_the_version_lacks_shows_a_dash_disabled() {
    let path = fixture(PesVersion::Pes15, "gated_stats_15");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    // A gated row may sit below the detail pane's fold: scroll each inside
    // the clip, where it takes a hover, before aiming the pointer.
    for (label, tooltip) in [("Physical Contact", "PES 17+"), ("Clearing", "PES 16+")] {
        let nodes = scroll_into_view(&mut window, &mut view, |node| {
            node.role() == Role::Label && node.value() == Some(label)
        });
        let button = disabled_dash(&nodes, label);
        assert_eq!(
            tooltip_after_hover(&mut window, &mut view, centre(button)),
            Some(tooltip.to_string()),
            "{label}'s tooltip"
        );
    }
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Speed")
    });
    let speed = right_of_label(&nodes, "Speed", Role::SpinButton)
        .into_iter()
        .next()
        .expect("the Speed control");
    assert!(!speed.is_disabled(), "Speed is enabled on PES 15");
}

// The label of a stat the version lacks paints the colour a version-gated
// card's name does, not the normal text colour; a stored stat's label
// paints the normal one on every version.
// SE-GUI-70
#[test]
fn a_stat_the_version_lacks_has_its_label_dimmed_as_a_gated_card() {
    let path = fixture(PesVersion::Pes17, "gated_stat_label");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let text_color = window.ctx.style_of(window.ctx.theme()).visuals.text_color();

    // A gated row may sit below the detail pane's fold: scroll each inside
    // the clip, then read the settled frame's shapes. `Through Passing`
    // (SKILLS[40]) is a card PES 17 lacks: its name's colour is the dimmed
    // one the gated stats' labels take.
    scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::CheckBox && node.label() == Some("Through Passing")
    });
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let dimmed = painted_checkbox_color(&nodes, &shapes, "Through Passing");
    assert_ne!(
        dimmed, text_color,
        "premise: a gated card's name is not the normal text colour"
    );

    for label in ["Tight Possession", "Aggression"] {
        scroll_into_view(&mut window, &mut view, |node| {
            node.role() == Role::Label && node.value() == Some(label)
        });
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert_eq!(
            painted_label_color(&nodes, &shapes, label),
            dimmed,
            "{label}'s label dimmed as a gated card's name is"
        );
        // The dimmed name answers a hover as its `-` and a gated card's name do.
        let name = nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(label))
            .expect("the gated stat's label");
        assert_eq!(
            tooltip_after_hover(&mut window, &mut view, centre(name)),
            Some("PES 20+".to_string()),
            "{label}'s name names the first version on hover"
        );
    }
    scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Speed")
    });
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        painted_label_color(&nodes, &shapes, "Speed"),
        text_color,
        "a stored stat's label in the normal text colour"
    );

    // On PES 21, which stores the two, they paint the normal text colour.
    let path = fixture(PesVersion::Pes21, "gated_stat_label_21");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    for label in ["Tight Possession", "Aggression"] {
        scroll_into_view(&mut window, &mut view, |node| {
            node.role() == Role::Label && node.value() == Some(label)
        });
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert_eq!(
            painted_label_color(&nodes, &shapes, label),
            text_color,
            "{label} stored on PES 21 paints the normal text colour"
        );
    }
}

// The ability stats sit in one column in the game's order, each label
// beside its number: the 25 stats run top to bottom, their labels'
// left edges at one x, each label's SpinButton on its row.
#[test]
fn the_stats_fill_one_column_in_the_games_order() {
    let path = fixture(PesVersion::Pes21, "stats_column");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = |label: &str| {
        nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(label))
            .unwrap_or_else(|| panic!("no label {label:?}"))
            .bounds()
            .expect("bounds")
    };
    let labels: Vec<String> = pes_savefile::interchange::team_toml::labels::ABILITY_STATS
        .iter()
        .map(|name| pes_savefile::interchange::team_toml::labels::title_case(name))
        .collect();
    let x0 = at(&labels[0]).x0;
    let mut top = f64::MIN;
    for label in &labels {
        let bounds = at(label);
        assert!(
            (bounds.x0 - x0).abs() < 1.0,
            "{label} shares the column's x0"
        );
        assert!(bounds.y0 > top, "{label} sits below the stat above");
        top = bounds.y0;
        assert!(
            !right_of_label(&nodes, label, Role::SpinButton).is_empty(),
            "{label}'s number beside it"
        );
    }
}

// The stats are named as the save's game names them: PES 2017 calls
// heading Header, PES 2021 keeps Heading. The Other stats read Injury
// Resistance and Weak Foot: Usage on both. The Skills panel's Heading is a
// card, a checkbox, so only the labels are read.
#[test]
fn the_stats_are_named_as_the_saves_game_names_them() {
    for (version, heading, other) in [
        (PesVersion::Pes17, "Header", "Heading"),
        (PesVersion::Pes21, "Heading", "Header"),
    ] {
        let path = fixture(version, &format!("stat_names_{}", version.number()));
        let mut view = SaveView::new();
        view.open(&path);
        let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert!(label_value(&nodes, heading), "{version:?}: {heading} drawn");
        assert!(!label_value(&nodes, other), "{version:?}: no {other} label");
        for label in ["Injury Resistance", "Weak Foot: Usage"] {
            assert!(label_value(&nodes, label), "{version:?}: {label} drawn");
        }
    }
}

// The ability stats' labels stand left-aligned in one column: every
// label's left edge at the label cell's, the cell padding (6) in from the
// panel's title, a short label (Jump) at the same x as a long one
// (Defensive Prowess). Every box starts at one x, 12 points after the
// widest label's right edge, and each label is centred on its box's row.
#[test]
fn the_stats_labels_start_at_one_x_and_their_boxes_at_another() {
    let path = fixture(PesVersion::Pes21, "stats_left_aligned");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels: Vec<String> = pes_savefile::interchange::team_toml::labels::ABILITY_STATS
        .iter()
        .map(|name| pes_savefile::interchange::team_toml::labels::title_case(name))
        .collect();
    let title = label_bounds(&nodes, "Ability stats").x0;
    let cell_left = title + 6.0;
    let jump = label_bounds(&nodes, "Jump");
    let defensive = label_bounds(&nodes, "Defensive Prowess");
    assert!(
        jump.x1 - jump.x0 < defensive.x1 - defensive.x0,
        "premise: Jump is the shorter label ({jump:?} vs {defensive:?})"
    );
    assert!(
        (jump.x0 - defensive.x0).abs() <= 0.5,
        "Jump starts where Defensive Prowess does ({} vs {})",
        jump.x0,
        defensive.x0
    );
    let widest = labels
        .iter()
        .map(|label| {
            let bounds = label_bounds(&nodes, label);
            bounds.x1 - bounds.x0
        })
        .fold(f64::MIN, f64::max);
    let box_left = cell_left + widest + 12.0;
    for label in &labels {
        let bounds = label_bounds(&nodes, label);
        let field = right_of_label(&nodes, label, Role::SpinButton)
            .first()
            .unwrap_or_else(|| panic!("{label}'s number"))
            .bounds()
            .expect("bounds");
        assert!(
            (bounds.x0 - cell_left).abs() <= 0.5,
            "{label}'s left edge {} at the label cell's {cell_left}",
            bounds.x0
        );
        assert!(
            (field.x0 - box_left).abs() <= 0.5,
            "{label}'s box at {}, 12 after the widest label at {box_left}",
            field.x0
        );
        assert!(
            ((bounds.y0 + bounds.y1) / 2.0 - (field.y0 + field.y1) / 2.0).abs() <= 0.5,
            "{label} centred on its box's row ({bounds:?} vs {field:?})"
        );
    }
}

// The ability stats' rows stand 22 apart, as the draft's grid rows do: an
// 18-point box with its cell's 2 points above and below.
#[test]
fn the_stats_rows_stand_22_apart() {
    let path = fixture(PesVersion::Pes21, "stats_row_pitch");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels: Vec<String> = pes_savefile::interchange::team_toml::labels::ABILITY_STATS
        .iter()
        .map(|name| pes_savefile::interchange::team_toml::labels::title_case(name))
        .collect();
    for pair in labels.windows(2) {
        let upper = label_bounds(&nodes, &pair[0]).y0;
        let lower = label_bounds(&nodes, &pair[1]).y0;
        assert!(
            (lower - upper - 22.0).abs() <= 0.5,
            "{} stands 22 below {}, not {}",
            pair[1],
            pair[0],
            lower - upper
        );
    }
}

// The playable positions' C/B/A ratings: a click stores its rating in the
// position's own `playable` slot, as one undoable edit.
#[test]
fn a_playable_positions_rating_writes_its_own_slot_and_undoes() {
    let path = fixture(PesVersion::Pes21, "playable_rating");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let playable = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .positions
            .playable
    };
    assert_eq!(playable(&view)[2], 0, "premise: LWF stores C");

    // LWF's three ratings in row order: C, B, A. Click A (stored 2;
    // `playable[2]` is LWF's array slot, not POSITIONS[9]'s index).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let ratings = rating_buttons(&nodes, "LWF");
    click(&mut window, ratings[2], &mut view);
    assert_eq!(
        playable(&view),
        {
            let mut expected = [0u8; 13];
            expected[2] = 2;
            expected
        },
        "A lands in LWF's slot and no other"
    );
    // One Ctrl+Z restores the entry.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(playable(&view)[2], 0, "the one undo restores it");
}

/// The narrowest label-to-field gap over `pairs` (`field.x0 − label.x1`
/// per row).
fn min_gap<S: AsRef<str>>(nodes: &[accesskit::Node], pairs: &[(S, Role)]) -> f64 {
    pairs
        .iter()
        .map(|(label, role)| {
            let label = label.as_ref();
            let label_rect = label_bounds(nodes, label);
            let field = right_of_label(nodes, label, *role)
                .first()
                .unwrap_or_else(|| panic!("a field right of {label:?}"))
                .bounds()
                .expect("bounds");
            field.x0 - label_rect.x1
        })
        .fold(f64::MAX, f64::min)
}

/// `label`'s label-node top, a shorthand for the row checks below.
fn y_of(nodes: &[accesskit::Node], label: &str) -> f64 {
    nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(label))
        .unwrap_or_else(|| panic!("no label {label:?}"))
        .bounds()
        .expect("bounds")
        .y0
}

/// The outline strokes' shapes, painted in the selection colour (the
/// registered position's cell mark, the draft's `.poscell.reg`).
fn outlines(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::epaint::RectShape> {
    let selection = egui::Visuals::dark().selection.stroke.color;
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == egui::Color32::TRANSPARENT && shape.stroke.color == selection =>
            {
                Some(shape.clone())
            }
            _ => None,
        })
        .collect()
}

/// The rect holding `position`'s cell widgets (its label over its C
/// to A ratings, the cell's own edges 3.5 out on each side).
fn cell_rect(nodes: &[accesskit::Node], position: &str) -> egui::Rect {
    let label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(position))
        .unwrap_or_else(|| panic!("no label {position:?}"));
    let ratings = rating_buttons(nodes, position);
    let (x0, x1, top) = (
        label.bounds().expect("bounds").x0 as f32,
        label.bounds().expect("bounds").x1 as f32,
        label.bounds().expect("bounds").y0 as f32,
    );
    egui::Rect::from_min_max(
        egui::pos2(x0.min(ratings[0].bounds().expect("bounds").x0 as f32), top),
        egui::pos2(
            x1.max(ratings[2].bounds().expect("bounds").x1 as f32),
            ratings[2].bounds().expect("bounds").y1 as f32,
        ),
    )
}

// The positions grid, the draft's `POS_ROWS` and `#pos-grid`: the 13
// playable positions in five rows by line of the pitch, left wing to
// right wing — LWF CF SS RWF / LMF AMF RMF / CMF DMF / LB CB RB / GK,
// GK alone on the last — each row centred in the column, each cell's
// label centred over its ratings (over the B), the registered position's
// cell outlined with the widgets' radius.
#[test]
fn the_positions_grid_draws_five_centered_rows_with_the_registered_outlined() {
    let path = fixture(PesVersion::Pes21, "positions_grid");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The five rows in the pitch's order, each on its own y, x order as
    // listed, GK alone on the last.
    const EXPECTED: [&[&str]; 5] = [
        &["LWF", "CF", "SS", "RWF"],
        &["LMF", "AMF", "RMF"],
        &["CMF", "DMF"],
        &["LB", "CB", "RB"],
        &["GK"],
    ];
    let mut last_y = f64::MIN;
    for row in EXPECTED {
        let row_y = y_of(&nodes, row[0]);
        assert!(
            row_y > last_y + 5.0,
            "the {row:?} row runs below the one before"
        );
        last_y = row_y;
        let mut last_x = f64::MIN;
        for position in row {
            let bounds = label_bounds(&nodes, position);
            assert!(
                (bounds.y0 - row_y).abs() < 1.0,
                "{position} sits on its row"
            );
            assert!(bounds.x0 > last_x, "{position} follows the one before");
            last_x = bounds.x0;
        }
    }

    // Each row centred in the column: the cells' span's centre at the
    // column's centre (the 62-wide cells' edges sit 3.5 outside the
    // centred ratings row, so the outside ratings' bounds bound the
    // span).
    let column_left = label_bounds(&nodes, "General info").x0;
    let column_centre = column_left + 350.0 / 2.0;
    for row in EXPECTED {
        let first_c = rating_buttons(&nodes, row[0])[0].bounds().expect("bounds");
        let last_a = rating_buttons(&nodes, row[row.len() - 1])[2]
            .bounds()
            .expect("bounds");
        let centre = (first_c.x0 + last_a.x1) / 2.0;
        assert!(
            (centre - column_centre).abs() < 1.0,
            "the {row:?} row centred in the column ({centre} vs {column_centre})"
        );
    }

    // Each cell's label centred over its ratings, over the B.
    for position in EXPECTED.into_iter().flatten() {
        let label = label_bounds(&nodes, position);
        let ratings = rating_buttons(&nodes, position);
        let b = ratings[1].bounds().expect("bounds");
        let centre = |rect: accesskit::Rect| (rect.x0 + rect.x1) / 2.0;
        assert!(
            (centre(label) - centre(b)).abs() < 1.0,
            "{position}'s label centred over its B"
        );
        assert!(
            label.y1 <= b.y0 + 1.0,
            "{position}'s label sits above its ratings"
        );
    }

    // The registered position's cell is the one outlined cell, stroked
    // with the widgets' own corner radius (every frame and field in the
    // view is rounded); the pick moves the outline with it (70101 is
    // registered CB, picked CF).
    let strokes = outlines(&shapes);
    assert_eq!(strokes.len(), 1, "exactly one outlined cell");
    let radius = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .widgets
        .inactive
        .corner_radius;
    assert_eq!(
        strokes[0].corner_radius, radius,
        "the outline rounded as the widgets are"
    );
    let cell = cell_rect(&nodes, "CB");
    assert!(
        strokes[0].rect.contains_rect(cell),
        "the outline is around the CB cell, got {:?} for {cell:?}",
        strokes[0].rect
    );

    let combo = right_of_label(&nodes, "Registered position", Role::ComboBox)
        .first()
        .copied()
        .expect("the registered combo");
    click(&mut window, combo, &mut view);
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (item, _) = nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button && node.label() == Some("CF - Centre Forward")
        })
        .expect("the CF item");
    action_click(&mut window, &mut view, *item);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let strokes = outlines(&shapes);
    assert_eq!(strokes.len(), 1, "still exactly one outlined cell");
    let cell = cell_rect(&nodes, "CF");
    assert!(
        strokes[0].rect.contains_rect(cell),
        "the outline moved to the CF cell, got {:?} for {cell:?}",
        strokes[0].rect
    );
}

// One undo entry per gesture, the merge key the widget's: the cards' and
// pickers' merge keys are the player's own — while a gesture stays open
// (a widget keeps the keyboard), a tick on the next player is his own
// entry, not merged into the first's.
#[test]
fn a_card_tick_on_another_player_is_its_own_undo_entry() {
    let path = fixture(PesVersion::Pes21, "cards_merge_key");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    view.selected = Some(Selection::Player(702, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let file = EditFile::load(&path).unwrap();
    let a_id = file.team(702).unwrap().roster[0].player_id;
    let b_id = file.team(702).unwrap().roster[1].player_id;
    let heading = |view: &SaveView, id: u32| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(id)
            .unwrap()
            .skills
            .skills[6]
    };
    let before_a = heading(&view, a_id);
    let before_b = heading(&view, b_id);

    // The undo gesture stays open while a widget holds the keyboard: focus
    // the filter box — it is drawn for every selection, so the entry
    // survives the player switch — then tick his card through AccessKit.
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (filter, _) = nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::TextInput
                && node.placeholder() == Some("Team or player: name, shirt name or ID")
        })
        .expect("the filter box");
    let (card_a, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::CheckBox && node.label() == Some("Heading"))
        .expect("the Heading card");
    action_focus(&mut window, &mut view, *filter);
    action_click(&mut window, &mut view, *card_a);
    assert_ne!(heading(&view, a_id), before_a, "the tick landed on him");

    // Select the next player while the gesture stays open (the filter
    // still has the keyboard), and tick the same card there: his own
    // entry, not a merge.
    view.selected = Some(Selection::Player(702, 1));
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (card_b, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::CheckBox && node.label() == Some("Heading"))
        .expect("the Heading card");
    action_click(&mut window, &mut view, *card_b);
    assert_ne!(heading(&view, b_id), before_b, "his tick landed too");

    // One undo reverts the second tick alone: the first player's card
    // stays as ticked, his own entry still on the stack.
    assert!(view.session.as_mut().unwrap().undo());
    assert_eq!(heading(&view, b_id), before_b, "the second tick undone");
    assert_ne!(
        heading(&view, a_id),
        before_a,
        "the first player's tick stands"
    );
    assert!(
        view.session.as_ref().unwrap().can_undo(),
        "the first tick is its own entry"
    );
}

// The fields shown one above their stored value (star and playing
// attitude are stored as shown): a stored 3 shows 4; typing 8 stores 7.
// The other fields of the same player are untouched by each edit. The Profile
// block's four sit in this view, the Cutscenes group's two on Appearance.
#[test]
fn the_shifted_fields_show_one_above_stored_and_store_one_below() {
    let path = fixture(PesVersion::Pes21, "shifted_fields");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    // The six fields the two groups hold: form, injury resistance, weak
    // foot usage and accuracy, star, playing attitude.
    let six = |view: &SaveView| {
        let stats = &view
            .session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .stats;
        (
            stats.form,
            stats.injury_resistance,
            stats.weak_foot_usage,
            stats.weak_foot_accuracy,
            stats.star,
            stats.playing_attitude,
        )
    };
    assert_eq!(
        six(&view),
        (3, 0, 1, 1, Some(0), Some(0)),
        "premise: the stored values"
    );

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let form = right_of_label(&nodes, "Form", Role::SpinButton)
        .first()
        .copied()
        .expect("the Form control");
    assert_eq!(form.value(), Some("4"), "stored 3 shows 4");
    click(&mut window, form, &mut view);
    window.frame(vec![egui::Event::Text("8".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        six(&view),
        (7, 0, 1, 1, Some(0), Some(0)),
        "typed 8 stores 7"
    );

    // The same round trip for the other five fields: each shows the
    // stored value plus its shift (star and playing attitude shift by
    // zero), a typed value stores one less, and the groups' other fields
    // keep their stored values. Star and playing attitude sit in the
    // Appearance section's Cutscenes group, editable while it is locked.
    for (appearance, label, shown, typed, expected) in [
        (
            false,
            "Injury Resistance",
            "1",
            "3",
            (7, 2, 1, 1, Some(0), Some(0)),
        ),
        (
            false,
            "Weak Foot: Usage",
            "2",
            "4",
            (7, 2, 3, 1, Some(0), Some(0)),
        ),
        (false, "Accuracy", "2", "4", (7, 2, 3, 3, Some(0), Some(0))),
        (true, "Star", "0", "7", (7, 2, 3, 3, Some(7), Some(0))),
        (
            true,
            "Playing attitude",
            "0",
            "3",
            (7, 2, 3, 3, Some(7), Some(3)),
        ),
    ] {
        let nodes = if appearance {
            open_appearance(&mut window, &mut view);
            // The Cutscenes group sits below the fold: the click needs
            // the control inside the scroll's clip.
            scroll_into_view(&mut window, &mut view, |node| {
                node.role() == Role::Label && node.value() == Some(label)
            })
        } else {
            let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
            nodes
        };
        let control = right_of_label(&nodes, label, Role::SpinButton)
            .first()
            .copied()
            .unwrap_or_else(|| panic!("the {label} control"));
        assert_eq!(control.value(), Some(shown), "{label} shows {shown}");
        click(&mut window, control, &mut view);
        window.frame(vec![egui::Event::Text(typed.to_owned())], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
        window.frame(
            vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
        assert_eq!(six(&view), expected, "{label} typed {typed}");
    }
}

// A value the save holds outside a control's range shows as stored and
// changes only on an edit — drawing a player never edits him.
#[test]
fn an_out_of_range_value_draws_as_stored_without_marking_the_player() {
    // Speed 30 is under the control's 40-99 range.
    let path = fixture(PesVersion::Pes21, "draws_no_edit");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().stats.speed = 30;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    for _ in 0..5 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let speed = right_of_label(&nodes, "Speed", Role::SpinButton)
        .first()
        .copied()
        .expect("the Speed control");
    assert_eq!(speed.value(), Some("30"), "the control draws the stored 30");
    assert_drawing_edited_nothing(&view);
    let session = view.session.as_ref().unwrap();
    assert_eq!(
        session.file().player(70101).unwrap().stats.speed,
        30,
        "the value stands as stored"
    );
}

// The same rule on a shifted field: Injury Resistance stores one under
// its shown 1-3, so a stored 3 draws the 4 it means, never clamped.
#[test]
fn an_out_of_range_injury_resistance_draws_as_stored_without_marking_the_player() {
    let path = fixture(PesVersion::Pes21, "injury_draws_no_edit");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    assert_ne!(
        file.player(70101).unwrap().stats.injury_resistance,
        3,
        "premise: Injury Resistance starts else"
    );
    file.player_mut(70101).unwrap().stats.injury_resistance = 3;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    for _ in 0..5 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let injury = right_of_label(&nodes, "Injury Resistance", Role::SpinButton)
        .first()
        .copied()
        .expect("the Injury Resistance control");
    assert_eq!(injury.value(), Some("4"), "the control draws the stored 3");
    assert_drawing_edited_nothing(&view);
    let session = view.session.as_ref().unwrap();
    assert_eq!(
        session
            .file()
            .player(70101)
            .unwrap()
            .stats
            .injury_resistance,
        3,
        "the value stands as stored"
    );
}

mod badges;
mod cards;
mod columns;
mod layout;
mod wheel;
