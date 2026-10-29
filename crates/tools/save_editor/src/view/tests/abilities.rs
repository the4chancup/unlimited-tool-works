use super::*;

/// The bounds of the Skills column's header (`ui.strong("Skills")`):
/// what the Cards left row's tests scope to.
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

/// The badge texts a frame's Cards left row drew (`"Regular: 3 of 7"`
/// or the weak `"No tier matches"`), in left-to-right order: the
/// Labels at the heading's height or below it of the badge's shapes
/// (labels of another shape must not pass).
fn cards_left_badges(nodes: &[accesskit::Node]) -> Vec<String> {
    let heading_y = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Label && node.value().is_some_and(|v| v.starts_with("Cards left"))
        })
        .and_then(accesskit::Node::bounds)
        .map(|bounds| bounds.y0)
        .expect("the Cards left heading");
    let mut badges: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Label
                && node
                    .bounds()
                    .is_some_and(|bounds| bounds.y0 >= heading_y - 1.0)
                && node.value().is_some_and(|value| {
                    // The badge's text is "tier: left of limit" or the weak
                    // stand-in; the "Free cards" panel's own title is not.
                    value == "No tier matches"
                        || value.rsplit_once(": ").is_some_and(|(_, count)| {
                            count.split_once(" of ").is_some_and(|(left, limit)| {
                                left.parse::<i32>().is_ok() && limit.parse::<u8>().is_ok()
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
    let reopened = Session::open(&path).unwrap();
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

// The ability stats sit in one column in the game's order, each label
// beside its number: the 25 stats run top to bottom, their labels'
// right edges at one x, each label's SpinButton on its row.
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
    let x1 = at(&labels[0]).x1;
    let mut top = f64::MIN;
    for label in &labels {
        let bounds = at(label);
        assert!(
            (bounds.x1 - x1).abs() < 1.0,
            "{label} shares the column's x1"
        );
        assert!(bounds.y0 > top, "{label} sits below the stat above");
        top = bounds.y0;
        assert!(
            !right_of_label(&nodes, label, Role::SpinButton).is_empty(),
            "{label}'s number beside it"
        );
    }
}

// The ability stats' labels stand right-aligned against their numbers:
// every label's right edge at one x, 12 points before its box, centred on
// its box's row, and the widest one starting the cell padding (6) in from
// the panel's title, where the labels started when left-aligned.
#[test]
fn the_stats_labels_stand_right_aligned_twelve_before_their_boxes() {
    let path = fixture(PesVersion::Pes21, "stats_right_aligned");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels: Vec<String> = pes_savefile::interchange::team_toml::labels::ABILITY_STATS
        .iter()
        .map(|name| pes_savefile::interchange::team_toml::labels::title_case(name))
        .collect();
    let right = label_bounds(&nodes, &labels[0]).x1;
    let mut left = f64::MAX;
    for label in &labels {
        let bounds = label_bounds(&nodes, label);
        let field = right_of_label(&nodes, label, Role::SpinButton)
            .first()
            .unwrap_or_else(|| panic!("{label}'s number"))
            .bounds()
            .expect("bounds");
        assert!(
            (bounds.x1 - right).abs() <= 0.5,
            "{label}'s right edge {} at the column's {right}",
            bounds.x1
        );
        assert!(
            (field.x0 - bounds.x1 - 12.0).abs() <= 0.5,
            "{label} ends 12 before its box ({} vs {})",
            bounds.x1,
            field.x0
        );
        assert!(
            ((bounds.y0 + bounds.y1) / 2.0 - (field.y0 + field.y1) / 2.0).abs() <= 0.5,
            "{label} centred on its box's row ({bounds:?} vs {field:?})"
        );
        left = left.min(bounds.x0);
    }
    let title = label_bounds(&nodes, "Ability stats").x0;
    assert!(
        (left - title - 6.0).abs() <= 0.5,
        "the leftmost label 6 in from the title ({left} vs {title})"
    );
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

// ST-WHEEL-01 in the editor: each wheel notch over the registered
// position's combo takes the item below, and each is its own undo step.
#[test]
fn each_wheel_notch_over_the_registered_position_is_one_undo_step() {
    let path = fixture(PesVersion::Pes21, "wheel_registered");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let registered = |view: &SaveView| player_entry(view, 70101).positions.registered;
    assert_eq!(registered(&view), 1, "premise: 70101 is registered CB");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = right_of_label(&nodes, "Registered position", Role::ComboBox)
        .first()
        .copied()
        .expect("the registered combo");
    let at = centre(combo);
    let notch_down = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    };
    // CB, then LB, then RB: the items below it in the list's order.
    for expected in [2, 3] {
        window.frame(
            vec![egui::Event::PointerMoved(at), notch_down.clone()],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
        assert_eq!(
            registered(&view),
            expected,
            "a notch down takes the next item"
        );
    }
    for expected in [2, 1] {
        window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
        assert_eq!(registered(&view), expected, "one Ctrl+Z undoes one notch");
    }
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the two notches were the only edits"
    );
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
                && node.placeholder() == Some("Filter every team (name, shirt name or id)")
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
            "Injury resistance",
            "1",
            "3",
            (7, 2, 1, 1, Some(0), Some(0)),
        ),
        (
            false,
            "Weak foot: Usage",
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
    let session = view.session.as_ref().unwrap();
    assert!(!session.is_dirty(), "drawing left no unsaved changes");
    assert!(!session.can_undo(), "drawing pushed no undo entry");
    assert_eq!(
        session.file().player(70101).unwrap().stats.speed,
        30,
        "the value stands as stored"
    );
}

// The Cards left badge is one badge, the player's own tier under the
// active ruleset (`label: left of limit`, `limit - paid`); a ticked
// skill that no free group covers lowers it by one; the active
// ruleset's reading is the badge's.
#[test]
fn the_cards_left_badge_reads_the_players_own_tier() {
    /// The badge text `rules` reads for team 701's roster `slot` on the
    /// session's file: his own tier's label over his card economy under
    /// it. The checked player must read as a tier that is not the
    /// ruleset's first — the badge then proves his own tier was read,
    /// not the list's head.
    fn expected(view: &SaveView, rules: &aatf::Rules, slot: u8) -> String {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 701).unwrap();
        let player = team.player(slot).unwrap();
        let tier = rules
            .player_tier(&team, player)
            .unwrap()
            .expect("the checked player matches a tier");
        assert_ne!(
            tier.id,
            rules.ruleset().tiers[0].id,
            "premise: slot {slot}'s own tier is not the ruleset's first"
        );
        let limits = rules.card_limits(&team, player, &tier.id).unwrap();
        format!(
            "{}: {} of {}",
            tier.label,
            i32::from(limits.limit) - i32::from(limits.paid),
            limits.limit
        )
    }
    /// The `left` an `expected` badge text carries (`"Regular: 3 of 7"`
    /// reads 3).
    fn left_of(view: &SaveView, rules: &aatf::Rules, slot: u8) -> i32 {
        expected(view, rules, slot)
            .split(": ")
            .nth(1)
            .expect("a badge text")
            .split(' ')
            .next()
            .unwrap()
            .parse()
            .expect("the left count")
    }

    let path = fixture(PesVersion::Pes21, "cards_left");
    // Slot 1's player holds Heading (skill 6), a paid card on every tier:
    // his badge is one lower than slot 0's unticked player's.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let second = file.team(701).unwrap().roster[1].player_id;
    file.player_mut(second).unwrap().skills.skills[6] = true;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let rules = aatf::Rules::official();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        cards_left_badges(&nodes),
        [expected(&view, &rules, 0)],
        "the one badge, his own tier's"
    );

    // "Heading" (skill 6) is unticked and counts paid on every tier: the
    // tick lowers the badge by one.
    let before = left_of(&view, &rules, 0);
    // The checkbox sits below the fold under the player header: the tick
    // goes through its AccessKit action (a pointer click lands only
    // inside the scroll's clip).
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (heading, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::CheckBox && node.label() == Some("Heading"))
        .expect("the Heading checkbox");
    action_click(&mut window, &mut view, *heading);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        cards_left_badges(&nodes),
        [expected(&view, &rules, 0)],
        "after the tick"
    );
    assert_eq!(left_of(&view, &rules, 0), before - 1, "one card left fewer");

    // Ctrl+Z unticks it: the badge is recomputed, back to the old count.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        cards_left_badges(&nodes),
        [expected(&view, &rules, 0)],
        "after the undo"
    );
    assert_eq!(left_of(&view, &rules, 0), before, "the badge restored");

    // The selection recomputes it: slot 1's player, his Heading held,
    // shows his own one-lower count — not slot 0's.
    view.selected = Some(Selection::Player(701, 1));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        cards_left_badges(&nodes),
        [expected(&view, &rules, 1)],
        "his badge, not slot 0's"
    );

    // The ruleset's reading is the badge's: the same view now set to DBC7
    // reloads on the changed setting (no fresh window).
    let dbc7 = aatf::builtin("dbc7").expect("DBC7 is built in");
    view.aatf_ruleset = "dbc7".to_owned();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        cards_left_badges(&nodes),
        [expected(&view, &dbc7, 1)],
        "the badge under DBC7"
    );
}

// The same bullet, painted: overdrawn past the tier's limit the badge is
// the error colour (slot 0's player made to hold paid cards past his own
// tier's limit by ticking skills in the fixture copy, as the Heading tick
// above does). A colour is no AccessKit property, so it is read back from
// the frame's painted shapes.
#[test]
fn an_overdrawn_badge_paints_in_the_error_colour() {
    /// The color the text `text` was painted in this frame (`galley_color`
    /// on its shape).
    fn painted_color(shapes: &[egui::epaint::ClippedShape], text: &str) -> egui::Color32 {
        let shape = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text == text => Some(shape),
                _ => None,
            })
            .next()
            .unwrap_or_else(|| panic!("no painted text {text:?}"));
        galley_color(shape)
    }

    let path = fixture(PesVersion::Pes21, "cards_left_overdrawn");
    // The first dozen skills ticked: enough paid cards that his own
    // tier's limit is passed.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let id = file.team(701).unwrap().roster[0].player_id;
    for skill in 0..12 {
        file.player_mut(id).unwrap().skills.skills[skill] = true;
    }
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let rules = aatf::Rules::official();
    let badge = {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 701).unwrap();
        let player = team.player(0).unwrap();
        let tier = rules
            .player_tier(&team, player)
            .unwrap()
            .expect("the player matches a tier");
        let limits = rules.card_limits(&team, player, &tier.id).unwrap();
        assert!(
            i32::from(limits.limit) - i32::from(limits.paid) < 0,
            "premise: overdrawn (paid {} of {})",
            limits.paid,
            limits.limit
        );
        format!(
            "{}: {} of {}",
            tier.label,
            i32::from(limits.limit) - i32::from(limits.paid),
            limits.limit
        )
    };
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let error_color = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .error_fg_color;
    assert_eq!(painted_color(&shapes, &badge), error_color);
}

// The same bullet: a player no tier's `when` holds for (`check_team`'s
// `NoTier`) gets the weak stand-in. VTL11's tiers each gate on an exact
// height (gold 195, silver 195, regular 180, buffed 188), so most real
// players match none — PES 21 fixture team 781 slot 0, player 78101.
#[test]
fn a_player_no_tier_matches_gets_the_stand_in() {
    let path = fixture(PesVersion::Pes21, "cards_left_no_tier");
    let rules = aatf::builtin("vtl11").expect("VTL11 is built in");
    let mut view = SaveView::new();
    view.open(&path);
    view.aatf_ruleset = "vtl11".to_owned();
    view.selected = Some(Selection::Player(781, 0));
    {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 781).unwrap();
        let player = team.player(0).unwrap();
        assert_eq!(
            rules.player_tier(&team, player).unwrap().map(|t| &t.id),
            None,
            "premise: 78101 matches no VTL11 tier"
        );
    }
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        cards_left_badges(&nodes),
        ["No tier matches"],
        "the weak stand-in, the one badge"
    );
}

// The same bullet: a ruleset that loads but whose `player_tier` fails on
// the selected player (a member's own logic edit) draws the red click line
// in the badge's place, not `No tier matches` (which would tell him the
// opposite), whose click opens the error; and the failure is kept under
// its key like a badge, so the failing script does not run again on every
// frame.
#[test]
fn a_failing_tier_check_shows_its_error_in_the_badges_place() {
    let path = fixture(PesVersion::Pes21, "cards_left_failing_check");
    let logic = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/aatf/logic/logic.rhai"),
    )
    .unwrap();
    let head = "fn player_tier(player, team) {\n";
    assert_eq!(logic.matches(head).count(), 1, "premise: one player_tier");
    let failing = logic.replacen(
        head,
        &format!("{head}    throw \"the tier check fails\";\n"),
        1,
    );
    let text = aatf::assemble_file(aatf::Rules::official().ruleset(), Some(&failing), None);
    let rules = aatf::Rules::load(&text).expect("premise: the file loads");
    let mut view = SaveView::new();
    view.open(&path);
    pick_imported(&mut view, "failing.4ccrules", &text);
    {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 701).unwrap();
        assert!(
            rules.player_tier(&team, team.player(0).unwrap()).is_err(),
            "premise: the check fails on the selected player"
        );
    }
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // `Cards left:`, then the red click line in the badge's place.
    const LINE: &str = "Tier check failed, click for more information.";
    let error_node = {
        let column_x0 = skills_header(&nodes).x0;
        nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value() == Some(LINE)
                    && node.bounds().is_some_and(|b| b.x0 >= column_x0 - 1.0)
            })
            .expect("the tier check's line in the badge's place")
    };
    let style = window.ctx.style_of(window.ctx.theme());
    assert_eq!(
        galley_color(text_shape(&shapes, LINE)),
        style.visuals.error_fg_color,
        "the line in the error colour"
    );
    assert!(
        in_skills_column(&nodes, "Cards left:"),
        "Cards left: heads it"
    );
    assert!(
        !in_skills_column(&nodes, "No tier matches"),
        "no stand-in beside it"
    );
    let expected = match view.cards.as_ref() {
        Some((_, crate::view::OwnBadge::Failed(error))) => error.clone(),
        _ => panic!("the failure is kept under its key"),
    };
    assert!(
        expected.contains("the tier check fails"),
        "premise: the check's error: {expected}"
    );

    // The dialog is up only after the click.
    assert!(
        !label_value(&nodes, "The tier check failed"),
        "no dialog title before the click"
    );
    assert!(view.refusal.is_none(), "no refusal before the click");

    // Its click opens the refusal dialog with the check's own error.
    click(&mut window, error_node, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "The tier check failed"),
        "the dialog's title"
    );
    assert!(
        label_value(&nodes, &expected),
        "the dialog's body is the check's error"
    );
}

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

/// `name`'s checkbox label as the cards' `title_case` draws it.
fn card_label(name: &str) -> String {
    pes_savefile::interchange::team_toml::labels::title_case(name)
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

// Every panel title of the view stands `TITLE_GAP` above the first thing
// drawn under it — a field's frame, a checkbox's painted square, the
// positions' label row's text — as the team view's titles do.
#[test]
fn every_panel_title_stands_six_above_its_first_drawn_row() {
    /// The first opaque rect shape under `title`'s label, in its column:
    /// the frame a field paints (`small`: a checkbox's painted square
    /// instead, the small rect inside the taller control).
    fn first_rect_under(
        nodes: &[accesskit::Node],
        shapes: &[egui::epaint::ClippedShape],
        title: &str,
        right: f64,
        small: bool,
    ) -> f64 {
        let title_bounds = label_bounds(nodes, title);
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.fill != egui::Color32::TRANSPARENT
                        && f64::from(shape.rect.min.y) > title_bounds.y1
                        && f64::from(shape.rect.min.x) >= title_bounds.x0 - 1.0
                        && f64::from(shape.rect.min.x) < right
                        && (!small
                            || (shape.rect.width() <= 15.0 && shape.rect.height() <= 15.0))
                        && (small || shape.rect.height() > 15.0) =>
                {
                    Some(f64::from(shape.rect.min.y))
                }
                _ => None,
            })
            .min_by(f64::total_cmp)
            .unwrap_or_else(|| panic!("no rect under {title:?}"))
    }
    /// The first text shape under `title`'s label, in its column.
    fn first_text_under(
        nodes: &[accesskit::Node],
        shapes: &[egui::epaint::ClippedShape],
        title: &str,
        right: f64,
    ) -> f64 {
        let title_bounds = label_bounds(nodes, title);
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape)
                    if f64::from(shape.pos.y) > title_bounds.y1
                        && f64::from(shape.pos.x) >= title_bounds.x0 - 1.0
                        && f64::from(shape.pos.x) < right =>
                {
                    Some(f64::from(shape.pos.y))
                }
                _ => None,
            })
            .min_by(f64::total_cmp)
            .unwrap_or_else(|| panic!("no text under {title:?}"))
    }

    let path = fixture(PesVersion::Pes21, "title_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    // The columns' widths settle over the first frames.
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The three columns' titles share one line.
    let titles = ["General info", "Skills", "Ability stats"].map(|t| label_bounds(&nodes, t));
    for pair in titles.windows(2) {
        assert!(
            (pair[0].y0 - pair[1].y0).abs() < 0.5,
            "the column titles' tops share a line: {titles:?}"
        );
    }
    let col2 = titles[1].x0;
    let col3 = titles[2].x0;
    let right = |title: &str| match title {
        "General info" | "Other stats" | "Playable positions" => col2,
        "Skills" | "Tricks" | "COM playing styles" => col3,
        _ => f64::MAX,
    };
    const GAP: f64 = 6.0;
    for (title, under) in [
        (
            "General info",
            first_rect_under(
                &nodes,
                &shapes,
                "General info",
                right("General info"),
                false,
            ),
        ),
        (
            "Other stats",
            first_rect_under(&nodes, &shapes, "Other stats", right("Other stats"), false),
        ),
        (
            "Playable positions",
            first_text_under(
                &nodes,
                &shapes,
                "Playable positions",
                right("Playable positions"),
            ),
        ),
        (
            "Skills",
            first_rect_under(&nodes, &shapes, "Skills", right("Skills"), true),
        ),
        (
            "Tricks",
            first_rect_under(&nodes, &shapes, "Tricks", right("Tricks"), true),
        ),
        (
            "COM playing styles",
            first_rect_under(
                &nodes,
                &shapes,
                "COM playing styles",
                right("COM playing styles"),
                true,
            ),
        ),
        (
            "Ability stats",
            first_rect_under(
                &nodes,
                &shapes,
                "Ability stats",
                right("Ability stats"),
                false,
            ),
        ),
    ] {
        let bottom = label_bounds(&nodes, title).y1;
        // The insets the panels subtract are estimates: a title may stand
        // a hair under the gap ("Playable positions" stands 5.84), never
        // over it.
        assert!(
            (-0.25..=0.0).contains(&(under - bottom - GAP)),
            "{title} stands {GAP} above its first drawn row ({under} vs {bottom})"
        );
    }
}

// The detail pane is one scroll area whatever row is picked: its offset
// outlives picking another player — a member editing one part of the
// Appearance player after player never scrolls back.
#[test]
fn the_detail_pane_keeps_its_scroll_offset_across_picks() {
    let path = fixture(PesVersion::Pes17, "scroll_offset_kept");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(734);
    view.selected = Some(Selection::Player(734, 0));
    // Shorter than the view is tall, so the pane scrolls.
    let mut window = Headless::sized(DEFAULT_PANE.x, 500.0);

    // Scroll the pane down to a deep stat: the view's top moves off.
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Aggression")
    });
    let scrolled = label_bounds(&nodes, "General info").y0;
    assert!(
        scrolled < 30.0,
        "premise: the pane scrolled (General info's top {scrolled})"
    );

    // Pick the next player: the same view height, so nothing clamps —
    // and the pane stays where it was.
    view.selected = Some(Selection::Player(734, 1));
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        (label_bounds(&nodes, "General info").y0 - scrolled).abs() < 0.5,
        "the offset outlives the pick ({scrolled} vs {})",
        label_bounds(&nodes, "General info").y0
    );
}

// The registered position combo stores the pick (POSITIONS index) as one
// undoable edit.
#[test]
fn the_registered_position_combo_stores_the_pick_as_one_edit() {
    let path = fixture(PesVersion::Pes21, "registered_position");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .positions
            .registered,
        1,
        "premise: registered CB"
    );

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = right_of_label(&nodes, "Registered position", Role::ComboBox)
        .first()
        .copied()
        .expect("the registered combo");
    assert_eq!(
        combo.value(),
        Some("CB - Centre Back"),
        "the combo names the position in full, British spelling"
    );
    click(&mut window, combo, &mut view);
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (item, _) = nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button && node.label() == Some("CF - Centre Forward")
        })
        .expect("the CF item");
    action_click(&mut window, &mut view, *item);
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .positions
            .registered,
        12,
        "CF stores 12"
    );
    // One gesture, one entry: a single undo brings CB back.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .positions
            .registered,
        1
    );
    assert!(!view.session.as_ref().unwrap().can_undo(), "one entry only");
}

// A click on a position's label registers it — the same undoable edit as
// General info's combo (the combo reads it, the cell takes the outline,
// the ratings untouched, the label's hover naming it); a click on the
// already registered position's label writes nothing.
// SE-GUI-40
#[test]
fn a_positions_label_click_registers_it_as_one_edit() {
    let path = fixture(PesVersion::Pes21, "registered_label_click");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let registered = |view: &SaveView| player_entry(view, 70101).positions.registered;
    let playable = |view: &SaveView| player_entry(view, 70101).positions.playable;
    assert_eq!(registered(&view), 1, "premise: registered CB");
    let ratings = playable(&view);
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "premise: a clean session"
    );

    // The label's hover names what its click does.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let ss = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("SS"))
        .expect("the SS label");
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            centre(ss),
            "the registered position"
        ),
        Some("Make SS the registered position".to_owned()),
        "the label's hover"
    );

    click(&mut window, ss, &mut view);
    assert_eq!(registered(&view), 11, "the click registered SS");
    assert_eq!(playable(&view), ratings, "every rating untouched");
    assert!(
        view.session.as_ref().unwrap().can_undo(),
        "the one undo entry"
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = right_of_label(&nodes, "Registered position", Role::ComboBox)
        .first()
        .copied()
        .expect("the registered combo");
    assert_eq!(
        combo.value(),
        Some("SS - Second Striker"),
        "the combo reads SS"
    );
    let strokes = outlines(&shapes);
    assert_eq!(strokes.len(), 1, "exactly one outlined cell");
    assert!(
        strokes[0].rect.contains_rect(cell_rect(&nodes, "SS")),
        "the outline is around the SS cell, got {:?}",
        strokes[0].rect
    );

    // One undo brings CB back; a click on the now-registered CB's label
    // writes nothing.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(registered(&view), 1, "the undo restores CB");
    assert!(!view.session.as_ref().unwrap().can_undo(), "one entry only");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let cb = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("CB"))
        .expect("the CB label");
    click(&mut window, cb, &mut view);
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the already-registered position's label writes no entry"
    );
    assert_eq!(registered(&view), 1, "and the registered stays CB");
}

// The same bullet's two-choice toggles (the draft's `.rate.lr`): Stronger
// foot and Stronger hand draw Left | Right as two equal halves of the
// row's field, the held side filled the positions grid's C grey
// (#5a5a5a) with the normal text colour, and a click on the other side
// stores the pick (FOOT index, right 0 left 1) as one undoable edit.
#[test]
fn the_foot_and_hand_sides_fill_the_held_side_and_store_the_pick() {
    let path = fixture(PesVersion::Pes21, "stronger_sides");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let positions =
        |view: &SaveView, side: fn(&PlayerEntry) -> Option<u8>| side(&player_entry(view, 70101));
    assert_eq!(
        positions(&view, |p| Some(p.positions.stronger_foot)),
        Some(0),
        "premise: right-footed"
    );
    assert_eq!(
        positions(&view, |p| p.positions.stronger_hand),
        Some(0),
        "premise: right-handed"
    );

    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let quiet_grey = egui::Color32::from_rgb(0x5a, 0x5a, 0x5a);
    for label in ["Stronger foot", "Stronger hand"] {
        let sides = right_of_label(&nodes, label, Role::Button);
        assert_eq!(sides.len(), 2, "{label} is two choices, not a combo");
        let (left, right) = (sides[0], sides[1]);
        assert_eq!(left.label(), Some("Left"));
        assert_eq!(right.label(), Some("Right"));
        let (left_b, right_b) = (
            left.bounds().expect("bounds"),
            right.bounds().expect("bounds"),
        );
        assert!(
            (left_b.x1 - left_b.x0 - (right_b.x1 - right_b.x0)).abs() < 0.5,
            "{label}'s choices are equal halves ({left_b:?} vs {right_b:?})"
        );
        assert_eq!(
            right.toggled(),
            Some(accesskit::Toggled::True),
            "{label} holds Right"
        );
        assert_eq!(
            fill_at(&shapes, right_b),
            Some(quiet_grey),
            "{label}'s held side fills the positions grid's C grey"
        );
        assert_eq!(
            left.toggled(),
            Some(accesskit::Toggled::False),
            "{label}'s other side is unheld"
        );
        click(&mut window, left, &mut view);
    }
    assert_eq!(
        positions(&view, |p| Some(p.positions.stronger_foot)),
        Some(1),
        "the foot's Left click stores 1"
    );
    assert_eq!(
        positions(&view, |p| p.positions.stronger_hand),
        Some(1),
        "the hand's Left click stores 1"
    );

    // One gesture each, one entry each: two undos bring both back.
    assert_eq!(undo_entries(&mut window, &mut view), 2, "one entry each");
    assert_eq!(
        positions(&view, |p| Some(p.positions.stronger_foot)),
        Some(0),
        "the foot undone"
    );
    assert_eq!(
        positions(&view, |p| p.positions.stronger_hand),
        Some(0),
        "the hand undone"
    );
}

// The frame that first shows a player is centred where every later frame
// centres him: its ability stats grid is new, so egui lays the frame's first
// pass out narrow and discards it, and the pass that reaches the screen
// must not centre on the widths that discarded pass measured.
#[test]
fn a_players_first_frame_draws_where_every_later_one_does() {
    let path = fixture(PesVersion::Pes17, "player_first_frame");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(734);
    view.selected = Some(Selection::Player(734, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }

    // Each pass of the frame selecting the next player: the columns' width
    // the centring pad reads, the width the pass measures, and whether egui
    // discards it.
    view.selected = Some(Selection::Player(734, 1));
    let mut passes = Vec::new();
    let (_, first, _) = window.frame(Vec::new(), |ui| {
        let read = crate::view::abilities::columns_width(ui);
        show(ui, &mut view, &|| Vec::new());
        let measured = crate::view::abilities::columns_width(ui);
        passes.push((read, measured, ui.ctx().will_discard()));
    });
    let (&(read, measured, _), discarded) = passes.split_last().expect("a frame runs a pass");
    assert!(
        discarded.iter().all(|&(_, _, discard)| discard),
        "only the last pass reaches the screen, of {passes:?}"
    );
    assert_eq!(read, measured, "the kept pass, of {passes:?}");

    let (_, later, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        label_bounds(&first, "General info").x0,
        label_bounds(&later, "General info").x0,
        "General info's x on the first frame and the next, of {passes:?}"
    );
}

// SE-GUI-37's player-view clause: General info, Skills and Ability stats
// side by side left to right on the pane the shell leaves at the 1280 x
// 780 window (`DEFAULT_PANE`), their tops level, Appearance closed below
// them inside the pane's height, every first-half box on its half's right
// edge and the second half's on its label column, every combo on the
// column's, and nothing past the pane's right edge.
// SE-GUI-37
#[test]
fn the_player_view_lays_out_its_three_columns_at_1280() {
    let path = fixture(PesVersion::Pes21, "player_columns");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    // The combos' label column is measured on the first frames: let it settle.
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The title line's first text is the player's id in the weak colour.
    let weak = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .weak_text_color();
    let title = text_shape(&shapes, "70101");
    assert_eq!(
        title.galley.job.sections.first().map(|s| {
            (
                s.format.color,
                &title.galley.job.text[s.byte_range.start.0..s.byte_range.end.0],
            )
        }),
        Some((weak, "70101 ")),
        "the title's first run is the weak id"
    );

    // The three columns' first panels: left edges increasing, tops level.
    let titles = ["General info", "Skills", "Ability stats"].map(|t| label_bounds(&nodes, t));
    assert!(
        titles[0].x0 < titles[1].x0 && titles[1].x0 < titles[2].x0,
        "General info, Skills, Ability stats left to right: {titles:?}"
    );
    assert!(
        (titles[0].y0 - titles[1].y0).abs() < 4.0 && (titles[1].y0 - titles[2].y0).abs() < 4.0,
        "the columns' tops level: {titles:?}"
    );

    // Appearance sits closed below the deepest panel's bottom: the last
    // ability stat's row or the positions grid's last, whichever is lower.
    let stat_bottom = right_of_label(&nodes, "Aggression", Role::SpinButton)[0]
        .bounds()
        .expect("bounds")
        .y1
        .max(label_bounds(&nodes, "Aggression").y1);
    let grid_bottom = rating_buttons(&nodes, "GK")
        .iter()
        .map(|node| node.bounds().expect("bounds").y1)
        .fold(0.0, f64::max);
    let deepest = stat_bottom.max(grid_bottom);
    let appearance = node_labeled(&nodes, Role::DisclosureTriangle, "Appearance");
    assert_eq!(
        appearance.is_expanded(),
        Some(false),
        "Appearance is closed"
    );
    assert!(
        appearance.bounds().expect("bounds").y0 >= deepest,
        "Appearance below the deepest panel's bottom ({deepest})"
    );
    // The view fits the pane with no scrolling: the header's bottom inside
    // the pane's height (its content edges less the two
    // `CentralPanel` margins, as `tactics.rs` bounds its deepest node).
    assert!(
        appearance.bounds().expect("bounds").y1 <= f64::from(DEFAULT_PANE.y - 2.0 * PANEL_MARGIN),
        "Appearance's bottom inside the pane ({:?} vs {})",
        appearance.bounds(),
        DEFAULT_PANE.y - 2.0 * PANEL_MARGIN
    );

    // The six label-left rows (Name, Shirt and the four combos) share
    // one label column: their fields start at one x and end flush with
    // the second half's boxes, not on the panel's edge.
    let name = name_field(&nodes).bounds().expect("bounds");
    let shirt = shirt_field(&nodes).bounds().expect("bounds");
    let shirt_label = label_bounds(&nodes, "Shirt number");
    let shirt_box = right_of_label(&nodes, "Shirt number", Role::SpinButton)[0]
        .bounds()
        .expect("bounds");
    let flush = shirt_box.x1;
    assert!(
        (name.x1 - flush).abs() < 0.5 && (shirt.x1 - flush).abs() < 0.5,
        "the counted fields end at the flush edge ({name:?}, {shirt:?} vs {flush})"
    );
    for label in ["Registered position", "Playing style"] {
        let combo = right_of_label(&nodes, label, Role::ComboBox)[0]
            .bounds()
            .expect("bounds");
        assert!(
            (combo.x0 - name.x0).abs() < 0.5,
            "{label} starts at the rows' x ({combo:?} vs {name:?})"
        );
        assert!(
            (combo.x1 - flush).abs() < 0.5,
            "{label} ends at the flush edge ({combo:?} vs {flush})"
        );
    }
    // The foot's and hand's Left | Right toggles span the same field.
    for label in ["Stronger foot", "Stronger hand"] {
        let sides = right_of_label(&nodes, label, Role::Button);
        assert_eq!(sides.len(), 2, "{label} is two choices");
        let span = (
            sides[0].bounds().expect("bounds").x0,
            sides[1].bounds().expect("bounds").x1,
        );
        assert!(
            (span.0 - name.x0).abs() < 0.5,
            "{label} starts at the rows' x ({span:?} vs {name:?})"
        );
        assert!(
            (span.1 - flush).abs() < 0.5,
            "{label} ends at the flush edge ({span:?} vs {flush})"
        );
    }
    // The counted labels sit on their field's row (the label's vertical
    // centre inside the field's bounds).
    for prefix in ["Name (", "Shirt ("] {
        let label = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value().is_some_and(|value| value.starts_with(prefix))
            })
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("no counted label {prefix:?}"));
        let field = if prefix.starts_with("Shirt") {
            shirt
        } else {
            name
        };
        assert!(
            (label.y0 + label.y1) / 2.0 >= field.y0 - 1.0
                && (label.y0 + label.y1) / 2.0 <= field.y1 + 1.0,
            "the {prefix}label on its field's row ({label:?} vs {field:?})"
        );
    }

    // The pairs' halves 28 points apart: the first half's boxes start at
    // the rows' fields' left edge (the first half is the rows' label
    // column, their 12 and a box), the second half's left edge 28 after
    // its box's right edge.
    let height_box = right_of_label(&nodes, "Height", Role::SpinButton)[0]
        .bounds()
        .expect("bounds");
    for (label, role) in [
        ("Height", Role::SpinButton),
        ("Weight", Role::SpinButton),
        ("Age", Role::SpinButton),
        ("Injury resistance", Role::SpinButton),
        ("Weak foot: Usage", Role::SpinButton),
    ] {
        let bounds = right_of_label(&nodes, label, role)[0]
            .bounds()
            .expect("bounds");
        assert!(
            (bounds.x0 - name.x0).abs() < 0.5,
            "{label}'s box starts at the rows' field x ({bounds:?} vs {name:?})"
        );
    }
    assert!(
        (shirt_label.x0 - height_box.x1 - 28.0).abs() < 0.5,
        "the second half 28 after the first ({shirt_label:?} vs {height_box:?})"
    );
    // The second half's boxes sit just after a label column as wide as
    // "Shirt number" and its 9-point gap (the draft's `72px 54px` grid,
    // the column's `column-gap`): one x across General info's and Other
    // stats' rows, so a short label (Nation, Form) keeps its box close.
    let box_x = shirt_label.x1 + 9.0;
    for (label, role) in [
        ("Shirt number", Role::SpinButton),
        ("Nation", Role::SpinButton),
        ("Form", Role::SpinButton),
        ("Accuracy", Role::SpinButton),
    ] {
        let bounds = right_of_label(&nodes, label, role)[0]
            .bounds()
            .expect("bounds");
        assert!(
            (bounds.x0 - box_x).abs() < 0.5,
            "{label}'s box just after the label column ({bounds:?} vs {box_x})"
        );
    }
    // The Captain radio centred in its 54-wide box (the draft's `.pair
    // > .box` `justify-content: center`), under the Nation box above it.
    let captain = right_of_label(&nodes, "Captain", Role::RadioButton)[0]
        .bounds()
        .expect("bounds");
    let nation = right_of_label(&nodes, "Nation", Role::SpinButton)[0]
        .bounds()
        .expect("bounds");
    assert!(
        ((captain.x0 + captain.x1) / 2.0 - (nation.x0 + nation.x1) / 2.0).abs() < 0.5,
        "the Captain radio centred under the Nation box ({captain:?} vs {nation:?})"
    );

    // A label and its field are never closer than these minimums: 9 on
    // every pair, 12 in the label-left rows and Ability stats, the
    // narrowest of each at its minimum.
    let pair_min = min_gap(
        &nodes,
        &[
            ("Height", Role::SpinButton),
            ("Weight", Role::SpinButton),
            ("Age", Role::SpinButton),
            ("Shirt number", Role::SpinButton),
            ("Nation", Role::SpinButton),
            ("Captain", Role::RadioButton),
            ("Injury resistance", Role::SpinButton),
            ("Weak foot: Usage", Role::SpinButton),
            ("Form", Role::SpinButton),
            ("Accuracy", Role::SpinButton),
        ],
    );
    assert!(
        (pair_min - 9.0).abs() < 0.5,
        "the narrowest pair gap at 9 ({pair_min})"
    );
    let mut row_min = min_gap(
        &nodes,
        &[
            ("Stronger foot", Role::Button),
            ("Stronger hand", Role::Button),
            ("Registered position", Role::ComboBox),
            ("Playing style", Role::ComboBox),
        ],
    );
    for prefix in ["Name (", "Shirt ("] {
        let label_rect = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value().is_some_and(|value| value.starts_with(prefix))
            })
            .and_then(|node| node.bounds())
            .expect("the counted label's bounds");
        let field = field_right_of_label(&nodes, prefix, Role::TextInput)
            .bounds()
            .expect("bounds");
        let gap = field.x0 - label_rect.x1;
        assert!(gap >= 11.5, "{prefix}gap {gap} under 12");
        row_min = row_min.min(gap);
    }
    assert!(
        (row_min - 12.0).abs() < 0.5,
        "the narrowest row gap at 12 ({row_min})"
    );
    let stat_pairs: Vec<(String, Role)> =
        pes_savefile::interchange::team_toml::labels::ABILITY_STATS
            .iter()
            .map(|name| {
                (
                    pes_savefile::interchange::team_toml::labels::title_case(name),
                    Role::SpinButton,
                )
            })
            .collect();
    let stat_min = min_gap(&nodes, &stat_pairs);
    assert!(
        (stat_min - 12.0).abs() < 0.5,
        "the narrowest stat gap at 12 ({stat_min})"
    );

    // The COM styles' second column starts at the skills' second column's x.
    let skills_top = titles[1].y0;
    let com_top = label_bounds(&nodes, "COM playing styles").y0;
    let checkbox_xs = |top: f64, bottom: f64| {
        let mut xs: Vec<f64> = nodes
            .iter()
            .filter(|node| node.role() == Role::CheckBox)
            .filter_map(|node| node.bounds())
            .filter(|b| b.y0 >= top && b.y0 < bottom)
            .map(|b| b.x0)
            .collect();
        xs.sort_by(f64::total_cmp);
        xs.dedup();
        xs
    };
    let skills_xs = checkbox_xs(skills_top, com_top);
    let com_xs = checkbox_xs(com_top, f64::MAX);
    assert_eq!(skills_xs.len(), 2, "the skills' two columns");
    assert_eq!(com_xs.len(), 2, "the COM styles' two columns");
    assert!(
        (skills_xs[1] - com_xs[1]).abs() < 1.0,
        "the second columns share an x ({skills_xs:?} vs {com_xs:?})"
    );

    // The columns one gap apart: Skills' left is General info's 350-point
    // column and a gap after its left (the fields' flush edge is the
    // measured label-left rows' width inside the column, so the gap to
    // the fields' edge moves with the body font), Ability stats' the same
    // gap after the cards' right edge.
    let after_general = titles[1].x0 - titles[0].x0 - 350.0;
    let cards_right = skills_xs[1] + f64::from(team_widgets::cards::CARD_COLUMN_WIDTH);
    let after_cards = titles[2].x0 - cards_right;
    assert!(
        (after_general - after_cards).abs() <= 0.5,
        "the columns one gap apart ({after_general} after General info, {after_cards} after \
         the cards)"
    );

    // Nothing drawn past the pane's right edge, the pane's right less
    // the two CentralPanel margins (`PANEL_MARGIN` each side).
    let pane_right = f64::from(DEFAULT_PANE.x - 2.0 * PANEL_MARGIN);
    for role in [
        Role::Button,
        Role::CheckBox,
        Role::ComboBox,
        Role::RadioButton,
        Role::SpinButton,
        Role::TextInput,
    ] {
        for node in nodes.iter().filter(|node| node.role() == role) {
            let bounds = node
                .bounds()
                .unwrap_or_else(|| panic!("{role:?} has no bounds"));
            assert!(
                bounds.x1 <= pane_right + 0.5,
                "{role:?} {} clips the pane's right edge ({bounds:?} vs {pane_right})",
                node.label().unwrap_or("?")
            );
        }
    }
}

// The room the three fixed columns leave is split into four equal gaps:
// after the player list, counted from its frame, between the columns, and
// after Ability stats to the window's edge; 29.4 points each at the
// 1280 x 780 window, and still equal on a wider one. The title line and
// Appearance start at General info's title.
// The gaps read the columns' widths the previous frame measured (egui
// memory), so three settle frames run before the read.
#[test]
fn the_player_view_splits_its_room_into_four_equal_gaps() {
    let path = fixture(PesVersion::Pes21, "view_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    for screen_w in [DEFAULT_PANE.x, DEFAULT_PANE.x + 300.0] {
        let mut window = Headless::sized(screen_w, DEFAULT_PANE.y);
        for _ in 0..3 {
            window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        }
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let widths: [f32; 3] = window
            .ctx
            .data(|data| data.get_temp(egui::Id::unique(crate::view::abilities::COLUMN_WIDTHS_ID)))
            .expect("the columns' measured widths");
        let frame_right = f64::from(list_frame(&window, &shapes).max.x);
        let lefts = ["General info", "Skills", "Ability stats"].map(|t| label_bounds(&nodes, t).x0);
        let gaps = [
            lefts[0] - frame_right,
            lefts[1] - lefts[0] - f64::from(widths[0]),
            lefts[2] - lefts[1] - f64::from(widths[1]),
            f64::from(screen_w) - lefts[2] - f64::from(widths[2]),
        ];
        let (least, most) = gaps.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &gap| {
            (lo.min(gap), hi.max(gap))
        });
        assert!(
            most - least <= 0.5,
            "the four gaps equal at {screen_w}: {gaps:?}"
        );
        if screen_w == DEFAULT_PANE.x {
            for gap in gaps {
                assert!(
                    (gap - 29.4).abs() <= 0.5,
                    "each gap 29.4 at the 1280 x 780 window: {gaps:?}, the frame's right {frame_right}, the columns {widths:?}"
                );
            }
        }

        // The title line and Appearance start at General info's title.
        let title = nodes
            .iter()
            .filter(|node| node.role() == Role::Label)
            .find(|node| {
                node.value()
                    .is_some_and(|value| value.starts_with("70101 "))
            })
            .and_then(accesskit::Node::bounds)
            .expect("the view's title");
        let appearance = node_labeled(&nodes, Role::DisclosureTriangle, "Appearance")
            .bounds()
            .expect("bounds");
        assert!(
            (title.x0 - lefts[0]).abs() <= 0.5 && (appearance.x0 - lefts[0]).abs() <= 0.5,
            "the title ({title:?}) and Appearance ({appearance:?}) start at {}",
            lefts[0]
        );
    }
}

// General info's and Other stats' fields and labels stand 6 in from
// their titles, as the ability stats stand in from theirs: the column
// 350 wide, the fields 344.
#[test]
fn general_info_and_other_stats_stand_six_in_from_their_titles() {
    let path = fixture(PesVersion::Pes21, "general_inset");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let general = label_bounds(&nodes, "General info").x0;
    let name = nodes
        .iter()
        .filter(|node| node.role() == Role::Label)
        .find(|node| {
            node.value()
                .is_some_and(|value| value.starts_with("Name ("))
        })
        .and_then(accesskit::Node::bounds)
        .expect("the Name label");
    for (what, left) in [
        ("Name's label", name.x0),
        ("Height's label", label_bounds(&nodes, "Height").x0),
        (
            "Stronger foot's label",
            label_bounds(&nodes, "Stronger foot").x0,
        ),
    ] {
        assert!(
            (left - general - 6.0).abs() <= 0.5,
            "{what} 6 in from General info's title ({left} vs {general})"
        );
    }
    let other = label_bounds(&nodes, "Other stats").x0;
    let injury = label_bounds(&nodes, "Injury resistance").x0;
    assert!(
        (injury - other - 6.0).abs() <= 0.5,
        "Injury resistance 6 in from Other stats' title ({injury} vs {other})"
    );
}

// The Ability stats column is the draft's 169.4 points: each row a label
// cell and a value cell with 6 points of padding a side (the draft's
// `stats-grid` cells), so the widest label stands 6 in from the title and
// the boxes end 6 short of the column's right edge.
#[test]
fn the_ability_stats_column_is_the_drafts_width() {
    let path = fixture(PesVersion::Pes21, "ability_column");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let widths: [f32; 3] = window
        .ctx
        .data(|data| data.get_temp(egui::Id::unique(crate::view::abilities::COLUMN_WIDTHS_ID)))
        .expect("the columns' measured widths");
    assert!(
        (widths[2] - 169.4).abs() <= 0.5,
        "the Ability stats column 169.4 wide ({widths:?})"
    );

    let title = label_bounds(&nodes, "Ability stats").x0;
    let widest = pes_savefile::interchange::team_toml::labels::ABILITY_STATS
        .iter()
        .map(|name| {
            label_bounds(
                &nodes,
                &pes_savefile::interchange::team_toml::labels::title_case(name),
            )
        })
        .max_by(|a, b| (a.x1 - a.x0).total_cmp(&(b.x1 - b.x0)))
        .expect("the stats' labels");
    assert!(
        (widest.x0 - title - 6.0).abs() <= 0.5,
        "the widest label 6 in from the title ({widest:?} vs {title})"
    );
    let boxes_right = nodes
        .iter()
        .filter(|node| node.role() == Role::SpinButton)
        .filter_map(accesskit::Node::bounds)
        .filter(|bounds| bounds.x0 >= title)
        .map(|bounds| bounds.x1)
        .fold(f64::MIN, f64::max);
    let column_right = title + f64::from(widths[2]);
    assert!(
        (column_right - boxes_right - 6.0).abs() <= 0.5,
        "the boxes end 6 short of the column's right edge ({boxes_right} vs {column_right})"
    );
}

/// The color the letter's galley painted inside `bounds`: a section's own
/// color when one was set, else the shape's fallback (the widget's).
fn letter_color(
    shapes: &[egui::epaint::ClippedShape],
    letter: &str,
    bounds: accesskit::Rect,
) -> egui::Color32 {
    let shape = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if shape.galley.job.text == letter
                    && f64::from(shape.pos.y) >= bounds.y0
                    && f64::from(shape.pos.y) <= bounds.y1
                    && f64::from(shape.pos.x) >= bounds.x0
                    && f64::from(shape.pos.x) <= bounds.x1 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted {letter:?} inside {bounds:?}"));
    galley_color(shape)
}

// A chosen C rating's square in the dark theme's grey (`#5a5a5a`) and in
// its light twin (`#9f9f9f`) on the light theme, where the light theme's
// dark letter still reads on it.
#[test]
fn a_chosen_c_rating_takes_each_themes_grey() {
    let path = fixture(PesVersion::Pes21, "rating_c_theme");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    // LWF at C (stored 0), its `playable` entry the third.
    file.player_mut(70101).unwrap().positions.playable[2] = 0;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    for (theme, grey) in [
        (egui::Theme::Dark, egui::Color32::from_gray(0x5a)),
        (egui::Theme::Light, egui::Color32::from_gray(0x9f)),
    ] {
        window.ctx.set_theme(theme);
        draw(&mut window, &mut view);
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let c = rating_buttons(&nodes, "LWF")[0];
        assert_eq!(
            c.toggled(),
            Some(accesskit::Toggled::True),
            "premise: LWF holds C"
        );
        assert_eq!(
            fill_at(&shapes, c.bounds().expect("bounds")),
            Some(grey),
            "LWF's C square on the {theme:?} theme"
        );
    }
}

// The chosen rating's fill and letter (the draft's `.sel.on` variants):
// A a deep green (`#129648`) under a white letter, B the selection colour with
// its foreground, C a grey square with the normal text, so the three
// tell apart at a glance.
#[test]
fn the_chosen_ratings_paint_their_fills_and_letters() {
    let path = fixture(PesVersion::Pes21, "rating_colors");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let mut playable = file.player(70101).unwrap().positions.playable;
    // CF at A (stored 2), SS at B (1), LWF at C (0), the first three
    // `playable` entries standing for them.
    assert_ne!(
        [playable[0], playable[1], playable[2]],
        [2, 1, 0],
        "premise: the fixture does not already hold A, B, C"
    );
    playable[0] = 2;
    playable[1] = 1;
    playable[2] = 0;
    file.player_mut(70101).unwrap().positions.playable = playable;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let cases = [
        (
            "CF",
            2,
            egui::Color32::from_rgb(0x12, 0x96, 0x48),
            egui::Color32::WHITE,
        ),
        (
            "SS",
            1,
            visuals.selection.bg_fill,
            visuals.selection.stroke.color,
        ),
        (
            "LWF",
            0,
            egui::Color32::from_rgb(0x5a, 0x5a, 0x5a),
            visuals.text_color(),
        ),
    ];
    for (position, rating, fill, text) in cases {
        let letter = ["C", "B", "A"][rating];
        let button = rating_buttons(&nodes, position)[rating];
        let bounds = button.bounds().expect("bounds");
        assert_eq!(
            fill_at(&shapes, bounds),
            Some(fill),
            "{position}'s {letter} square fills {fill:?}"
        );
        assert_eq!(
            letter_color(&shapes, letter, bounds),
            text,
            "{position}'s {letter} letter in {text:?}"
        );
    }
}
