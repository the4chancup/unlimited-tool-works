use super::*;

// SE-GUI-37's player-view clause: General info, Skills and Ability stats
// side by side left to right on the pane the shell leaves at the 1280 x
// 780 window (`DEFAULT_PANE`), their tops level, every first-half box on
// its half's right edge and the second half's on its label column, every
// combo on the column's, and nothing past the pane's right edge. The
// Stats screen carries no Appearance header under them (SE-GUI-82: the
// section is a screen of its own). PES 17's gated fields keep the layout.
// SE-GUI-37
#[test]
fn the_player_view_lays_out_its_three_columns_at_1280() {
    for version in [PesVersion::Pes21, PesVersion::Pes17] {
        assert_three_columns_at_1280(version);
    }
}

/// The player-view layout checks on `version`'s fixture save, its first
/// player shown.
fn assert_three_columns_at_1280(version: PesVersion) {
    let path = fixture(version, &format!("player_columns_{version:?}"));
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

    // Nothing draws below the deepest panel's bottom: the last ability
    // stat's row or the positions grid's last, whichever is lower, inside
    // the pane's height (its content edges less the two `CentralPanel`
    // margins, as `tactics.rs` bounds its deepest node). Aggression's
    // control is its number box, or PES 17's gated stand-in.
    let stat_bottom = control_right_of(&nodes, "Aggression")
        .bounds()
        .expect("bounds")
        .y1
        .max(label_bounds(&nodes, "Aggression").y1);
    let grid_bottom = rating_buttons(&nodes, "GK")
        .iter()
        .map(|node| node.bounds().expect("bounds").y1)
        .fold(0.0, f64::max);
    let deepest = stat_bottom.max(grid_bottom);
    assert!(
        deepest <= f64::from(DEFAULT_PANE.y - 2.0 * PANEL_MARGIN),
        "the deepest panel's bottom inside the pane ({deepest} vs {})",
        DEFAULT_PANE.y - 2.0 * PANEL_MARGIN
    );
    // The Stats screen carries no Appearance header: the section is a
    // screen of its own, on the title row's toggle.
    assert!(
        !nodes
            .iter()
            .any(|node| node.role() == Role::DisclosureTriangle),
        "no Appearance header under the columns"
    );

    // The six label-left rows (Name, Shirt name and the four combos) share
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
    for prefix in ["Name (", "Shirt name ("] {
        let label = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value().is_some_and(|value| value.starts_with(prefix))
            })
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("no counted label {prefix:?}"));
        let field = if prefix.starts_with("Shirt name") {
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

    // The pairs' halves at least 18 apart: the first half's boxes start
    // at the rows' fields' left edge (the first half is the rows' label
    // column, their 20 and a box), the second half's left edge 18 or
    // more after its box's right edge.
    let height_box = right_of_label(&nodes, "Height", Role::SpinButton)[0]
        .bounds()
        .expect("bounds");
    for (label, role) in [
        ("Height", Role::SpinButton),
        ("Weight", Role::SpinButton),
        ("Age", Role::SpinButton),
        ("Injury Resistance", Role::SpinButton),
        ("Weak Foot: Usage", Role::SpinButton),
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
        shirt_label.x0 - height_box.x1 >= 17.5,
        "the second half at least 18 after the first ({shirt_label:?} vs {height_box:?})"
    );
    // The second half's boxes sit just after a label column as wide as
    // "Shirt number" and its 20-point gap (the draft's `72px 54px` grid,
    // the column's `column-gap`): one x across General info's and Other
    // stats' rows, so a short label (Nation, Form) keeps its box close.
    let box_x = shirt_label.x1 + 20.0;
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

    // A label and its field are never closer than 20: on the pairs, in
    // the label-left rows and in Ability stats, the narrowest of each at
    // its minimum.
    let pair_min = min_gap(
        &nodes,
        &[
            ("Height", Role::SpinButton),
            ("Weight", Role::SpinButton),
            ("Age", Role::SpinButton),
            ("Shirt number", Role::SpinButton),
            ("Nation", Role::SpinButton),
            ("Captain", Role::RadioButton),
            ("Injury Resistance", Role::SpinButton),
            ("Weak Foot: Usage", Role::SpinButton),
            ("Form", Role::SpinButton),
            ("Accuracy", Role::SpinButton),
        ],
    );
    assert!(
        (pair_min - 20.0).abs() < 0.5,
        "the narrowest pair gap at 20 ({pair_min})"
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
    for prefix in ["Name (", "Shirt name ("] {
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
        assert!(gap >= 19.5, "{prefix}gap {gap} under 20");
        row_min = row_min.min(gap);
    }
    assert!(
        (row_min - 20.0).abs() < 0.5,
        "the narrowest row gap at 20 ({row_min})"
    );
    let stat_pairs: Vec<(String, Role)> =
        pes_savefile::interchange::team_toml::labels::ABILITY_STATS
            .iter()
            .zip(player_entry(&view, 70101).stats.abilities())
            .map(|(name, stored)| {
                // A stat the version does not store keeps its cell as
                // the gated `-` button.
                let role = if stored.is_some() {
                    Role::SpinButton
                } else {
                    Role::Button
                };
                (
                    pes_savefile::interchange::team_toml::labels::stat_name(version, name),
                    role,
                )
            })
            .collect();
    let stat_min = min_gap(&nodes, &stat_pairs);
    assert!(
        (stat_min - 20.0).abs() < 0.5,
        "the narrowest stat gap at 20 ({stat_min})"
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

    // The columns stand 8 apart: Skills' frame starts 8 after General
    // info's ends, Ability stats' 8 after Skills' ends.
    let frame = |title: &str| group_frame(&window, &nodes, &shapes, title);
    let (general, skills, ability) = (
        frame("General info"),
        frame("Skills"),
        frame("Ability stats"),
    );
    for (left, right) in [(general, skills), (skills, ability)] {
        assert!(
            (right.min.x - left.max.x - 8.0).abs() <= 0.5,
            "the columns 8 apart ({left:?} then {right:?})"
        );
    }

    // Nothing drawn past the pane's right edge.
    let pane_right = f64::from(PANE_RIGHT);
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

// Every field of General info's and Other stats' panels ends 12 inside
// its frame on the right (the content's own 6-point padding, the frame's
// 6 and its point of stroke): the counted fields, the foot toggle, the
// combos and both halves' boxes on one flush x, in both panels alike. At
// a window 300 wider the column takes the room, the pairs' halves stand
// 300 further apart and the first half's boxes have not moved.
#[test]
fn the_panels_fields_end_twelve_inside_their_frames_at_any_width() {
    let mut halves_gap: Option<f64> = None;
    let mut first_boxes = Vec::new();
    for wider in [0.0_f32, 300.0] {
        let (mut view, mut window) =
            positions_view(&format!("fields_flush_{wider:.0}"), DEFAULT_PANE.x + wider);
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let frame =
            |title: &str| f64::from(group_frame(&window, &nodes, &shapes, title).max.x) - 13.0;
        let (general, other) = (frame("General info"), frame("Other stats"));
        let right = |node: &accesskit::Node| node.bounds().expect("bounds").x1;
        let box_right = |label: &str, role: Role| {
            right_of_label(&nodes, label, role)
                .first()
                .copied()
                .unwrap_or_else(|| panic!("the {label} field"))
                .bounds()
                .expect("bounds")
                .x1
        };
        let foot = right_of_label(&nodes, "Stronger foot", Role::Button)
            .last()
            .copied()
            .expect("the foot toggle's right side");
        for (what, field_x1, edge) in [
            ("the Name field", right(name_field(&nodes)), general),
            ("the Shirt name field", right(shirt_field(&nodes)), general),
            ("the Stronger foot toggle", right(foot), general),
            (
                "the Registered position combo",
                box_right("Registered position", Role::ComboBox),
                general,
            ),
            (
                "the Shirt number box",
                box_right("Shirt number", Role::SpinButton),
                general,
            ),
            (
                "the Nation box",
                box_right("Nation", Role::SpinButton),
                general,
            ),
            ("the Form box", box_right("Form", Role::SpinButton), other),
            (
                "the Accuracy box",
                box_right("Accuracy", Role::SpinButton),
                other,
            ),
        ] {
            assert!(
                (field_x1 - edge).abs() <= 0.5,
                "{what} ends 12 inside its frame ({field_x1} vs {edge} at {wider})"
            );
        }
        // The halves' gap and the first half's boxes' left edges.
        let shirt_label = label_bounds(&nodes, "Shirt number");
        let height_box = right_of_label(&nodes, "Height", Role::SpinButton)[0]
            .bounds()
            .expect("bounds");
        let gap = shirt_label.x0 - height_box.x1;
        if let Some(before) = halves_gap {
            assert!(
                (gap - before - 300.0).abs() <= 0.5,
                "the halves 300 further apart ({gap} vs {before})"
            );
        }
        halves_gap = Some(gap);
        let boxes: Vec<f64> = [
            ("Height", Role::SpinButton),
            ("Weight", Role::SpinButton),
            ("Age", Role::SpinButton),
            ("Injury Resistance", Role::SpinButton),
            ("Weak Foot: Usage", Role::SpinButton),
        ]
        .iter()
        .map(|(label, role)| {
            right_of_label(&nodes, label, *role)[0]
                .bounds()
                .expect("bounds")
                .x0
        })
        .collect();
        if !first_boxes.is_empty() {
            for (label, (before, now)) in first_boxes.iter().zip(&boxes).enumerate() {
                assert!(
                    (now - before).abs() <= 0.5,
                    "the first half's box {label} has not moved ({now} vs {before} at {wider})"
                );
            }
        }
        first_boxes = boxes;
    }
}

/// The Stats screen's frames for the selected player at `width` and the
/// Appearance screen's for the same player after it: `(stats frames by
/// title, appearance's Strip style frame)` — Strip style spans the
/// screen's own edges.
fn stats_frames(
    test: &str,
    width: f32,
) -> (std::collections::HashMap<String, egui::Rect>, egui::Rect) {
    let (mut view, mut window) = positions_view(test, width);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let stats = [
        "General info",
        "Other stats",
        "Playable positions",
        "Skills",
        "Tricks",
        "COM playing styles",
        "Ability stats",
    ]
    .map(|title| {
        (
            title.to_owned(),
            group_frame(&window, &nodes, &shapes, title),
        )
    });
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    (
        stats.into_iter().collect(),
        group_frame(&window, &nodes, &shapes, "Strip style"),
    )
}

// The Stats screen's frames span the pane as the Appearance screen's
// groups' do: the leftmost frame's left edge and the rightmost's right
// edge on the groups' own, the columns and the panels of a column 8
// apart — and General info's column alone takes a wider window's extra
// room (the draft's `1fr`), the two fixed columns keeping their widths.
// The title's row still starts at the pane's left edge, the toggle first.
#[test]
fn the_stats_screens_frames_span_the_pane_as_appearances_do() {
    let mut span = Vec::new();
    for wider in [0.0_f32, 300.0] {
        let (frames, strip) =
            stats_frames(&format!("stats_span_{wider:.0}"), DEFAULT_PANE.x + wider);
        let general = frames["General info"];
        let ability = frames["Ability stats"];
        assert!(
            (general.min.x - strip.min.x).abs() <= 0.5
                && (ability.max.x - strip.max.x).abs() <= 0.5,
            "the columns' span on the groups' edges ({general:?}..{ability:?} vs {strip:?} at \
             {wider})"
        );
        for pair in ["General info", "Skills", "Ability stats"].windows(2) {
            let (left, right) = (frames[pair[0]], frames[pair[1]]);
            assert!(
                (right.min.x - left.max.x - 8.0).abs() <= 0.5,
                "the columns 8 apart ({left:?} then {right:?} at {wider})"
            );
        }
        for pair in [
            ["General info", "Other stats"],
            ["Other stats", "Playable positions"],
            ["Skills", "Tricks"],
            ["Tricks", "COM playing styles"],
        ] {
            let (above, below) = (frames[pair[0]], frames[pair[1]]);
            assert!(
                (below.min.y - above.max.y - 8.0).abs() <= 0.5,
                "the panels of a column 8 apart ({above:?} then {below:?} at {wider})"
            );
        }
        span.push((general.width(), frames["Skills"].width(), ability.width()));
    }
    assert!(
        (span[1].0 - span[0].0 - 300.0).abs() <= 0.5,
        "General info's frame 300 wider ({span:?})"
    );
    assert!(
        (span[1].1 - span[0].1).abs() <= 0.5 && (span[1].2 - span[0].2).abs() <= 0.5,
        "the fixed columns keep their widths ({span:?})"
    );

    // The title's row starts at the pane's left edge with the toggle.
    let path = fixture(PesVersion::Pes21, "stats_span_title");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let pane_left = f64::from(pane_frame(&shapes, window.screen).min.x + PANEL_MARGIN);
    let stats = node_labeled(&nodes, Role::Button, "Stats")
        .bounds()
        .expect("the Stats segment");
    assert!(
        (stats.x0 - pane_left).abs() <= 0.5,
        "the title's row at the pane's left ({stats:?} vs {pane_left})"
    );
}

// A window resize runs no discarded pass on the Stats screen: the
// stored widths are the two fixed columns' alone, so General info's
// flexible width is computed inside the pass that draws it rather than
// discarded as stale a frame later.
#[test]
fn a_window_resize_asks_for_no_discarded_pass() {
    let (mut view, mut window) = positions_view("resize_no_discard", DEFAULT_PANE.x);
    window.screen.x += 300.0;
    let mut discards = Vec::new();
    window.frame(Vec::new(), |ui| {
        show(ui, &mut view, &|| Vec::new());
        discards.push(ui.ctx().will_discard());
    });
    assert!(
        discards.iter().all(|discard| !discard),
        "no pass asked for a discard on the resize: {discards:?}"
    );
}

// The three columns' first frames' tops stand where the Appearance
// screen's first groups' do (`GROUP_GAP` under the title's line), so a
// switch between the screens moves no frame line.
#[test]
fn the_columns_first_frames_stand_where_appearances_groups_do() {
    let (mut view, mut window) = positions_view("stats_frame_tops", DEFAULT_PANE.x);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let tops = ["General info", "Skills", "Ability stats"]
        .map(|title| group_frame(&window, &nodes, &shapes, title).min.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let group_top = group_frame(&window, &nodes, &shapes, "Full Player Customization")
        .min
        .y;
    for (title, top) in ["General info", "Skills", "Ability stats"].iter().zip(tops) {
        assert!(
            (top - group_top).abs() <= 0.5,
            "{title}'s frame top on the groups' line ({top} vs {group_top})"
        );
    }
}

// The three columns end on one line: each column's last frame runs down
// to the tallest column's bottom (the draft's `.pcol > :last-child {
// flex: 1 }`).
#[test]
fn the_columns_last_frames_end_on_one_line() {
    let (mut view, mut window) = positions_view("stats_frame_bottoms", DEFAULT_PANE.x);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = |title: &str| group_frame(&window, &nodes, &shapes, title);
    let bottoms = [
        frame("Playable positions").max.y,
        frame("COM playing styles").max.y,
        frame("Ability stats").max.y,
    ];
    // The premise: the columns' contents differ in height, so the line
    // is a stretch's work and not three equal columns by chance.
    let aggression = label_bounds(&nodes, "Aggression").y1;
    let com_title = label_bounds(&nodes, "COM playing styles");
    let com_bottom = nodes
        .iter()
        .filter(|node| node.role() == Role::CheckBox)
        .filter_map(accesskit::Node::bounds)
        .filter(|b| b.y0 > com_title.y0)
        .map(|b| b.y1)
        .fold(0.0, f64::max);
    let gk = rating_buttons(&nodes, "GK")
        .iter()
        .map(|node| node.bounds().expect("bounds").y1)
        .fold(0.0, f64::max);
    let contents = [gk, com_bottom, aggression];
    let (least, most) = contents
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &y| (lo.min(y), hi.max(y)));
    assert!(
        most - least > 1.0,
        "premise: the columns' contents differ in height ({contents:?})"
    );
    assert!(
        (bottoms[0] - bottoms[1]).abs() <= 0.5 && (bottoms[1] - bottoms[2]).abs() <= 0.5,
        "the last frames' bottoms on one line ({bottoms:?})"
    );
}

// The player view's pane ends where the draft's `.detail` content does,
// at the view's right edge, 8 points short of the window's: with
// Appearance open the pane scrolls, and its floating bar's right edge is
// the pane's.
#[test]
fn the_player_views_pane_ends_8_from_the_windows_edge() {
    let path = fixture(PesVersion::Pes21, "player_pane_right");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    // The default width, and short enough that the Appearance screen,
    // whole at the default window, scrolls.
    let mut window = Headless::sized(DEFAULT_PANE.x, 600.0);
    open_appearance(&mut window, &mut view);
    // Three frames: the scroll's claimed content size lands a frame late.
    let mut nodes = Vec::new();
    for _ in 0..3 {
        let (_, frame_nodes, _) =
            window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        nodes = frame_nodes;
    }
    let bar = nodes
        .iter()
        .filter(|node| node.role() == Role::ScrollBar)
        .filter_map(accesskit::Node::bounds)
        .max_by(|a, b| a.x1.total_cmp(&b.x1))
        .expect("the detail pane's scroll bar");
    assert!(
        bar.x0 > 300.0,
        "premise: the rightmost bar is the detail pane's, not the list's ({bar:?})"
    );
    assert!(
        (bar.x1 - f64::from(PANE_RIGHT)).abs() <= 1.0,
        "the pane's right edge at {PANE_RIGHT}, the draft's 1272 ({bar:?})"
    );
}

// Every panel's content, its first column's labels and controls alike,
// stands 6 in from the panel's title, as the draft draws it: General
// info's fields, the positions' layout toggle, the cards' first column and
// the ability stats' labels. The title then reads as the heading of what
// is under it.
#[test]
fn every_panels_content_stands_six_in_from_its_title() {
    let path = fixture(PesVersion::Pes21, "panel_content_inset");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let columns: [&[&str]; 3] = [
        &["General info", "Other stats", "Playable positions"],
        &["Skills", "Tricks", "COM playing styles"],
        &["Ability stats"],
    ];
    let lefts = columns.map(|panels| label_bounds(&nodes, panels[0]).x0);
    let mut wrong = Vec::new();
    for (column, panels) in columns.iter().enumerate() {
        let right = lefts.get(column + 1).copied().unwrap_or(f64::MAX);
        for (at, panel) in panels.iter().enumerate() {
            let title = label_bounds(&nodes, panel);
            let bottom = panels
                .get(at + 1)
                .map_or(f64::MAX, |next| label_bounds(&nodes, next).y0);
            let (node, left) = nodes
                .iter()
                .filter_map(|node| node.bounds().map(|bounds| (node, bounds)))
                .filter(|(_, bounds)| {
                    *bounds != title
                        && bounds.y0 >= title.y1 - 0.5
                        && bounds.y1 <= bottom
                        && bounds.x0 >= title.x0 - 6.0
                        && bounds.x0 < right
                        && bounds.x1 > bounds.x0
                })
                .map(|(node, bounds)| (node, bounds.x0))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap_or_else(|| panic!("a node under {panel:?}"));
            let inset = left - title.x0;
            if (inset - 6.0).abs() > 0.5 {
                wrong.push(format!(
                    "{panel}: {:?} {:?} stands {inset} in",
                    node.role(),
                    node.label().or(node.value())
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

// The Ability stats column is the draft's 191.4 points: the stats grid's
// 177.4 of cells (each row a label cell and a value cell with 6 points of
// padding a side, the draft's `stats-grid`) inside the 6-point group
// frame, so the widest label stands 6 in from the title and the boxes
// end 13 short of the frame's right edge (the cell's 6, the frame's own
// 6 and its point of stroke).
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
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let widths: [f32; 2] = window
        .ctx
        .data(|data| data.get_temp(egui::Id::unique(crate::view::abilities::COLUMN_WIDTHS_ID)))
        .expect("the fixed columns' measured widths");
    assert!(
        (widths[1] - 191.4).abs() <= 0.5,
        "the Ability stats column 191.4 wide ({widths:?})"
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
    let frame_right = f64::from(group_frame(&window, &nodes, &shapes, "Ability stats").max.x);
    assert!(
        (frame_right - boxes_right - 13.0).abs() <= 0.5,
        "the boxes end 13 short of the frame's right edge ({boxes_right} vs {frame_right})"
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
        let ratings = rating_buttons(&nodes, position);
        let expected: Vec<Option<accesskit::Toggled>> = (0..3)
            .map(|index| {
                Some(if index == rating {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                })
            })
            .collect();
        assert_eq!(
            ratings
                .iter()
                .map(|node| node.toggled())
                .collect::<Vec<_>>(),
            expected,
            "{position}: only the chosen square reports selected"
        );
        let button = ratings[rating];
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

// The ratings' squares edge the same way at rest: the chosen one's edge
// its own fill's (`#30b466` on A's green, the selection blue's `#1e7a9e`
// on B, `#787878` on C's quiet grey), an unchosen one's the panel's quiet
// edge (`#2a2a2a`). The positions layout's held side, filled in the same
// quiet grey, edges `#787878` too.
#[test]
fn a_positions_cells_ratings_edge_by_their_fills_and_the_layout_toggle() {
    let path = fixture(PesVersion::Pes21, "rating_edges");
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
    let palette = studio_core::theme::palette(&visuals);
    let (edge_of, unheld) = (
        |fill| studio_core::theme::edge_color(&visuals, fill),
        studio_core::theme::unheld_edge(&visuals),
    );
    assert_eq!(
        (
            edge_of(palette.rating_a_fill),
            edge_of(visuals.selection.bg_fill),
            edge_of(palette.rating_c_fill),
            unheld,
        ),
        (
            egui::Color32::from_rgb(0x30, 0xb4, 0x66),
            egui::Color32::from_rgb(0x1e, 0x7a, 0x9e),
            egui::Color32::from_rgb(0x78, 0x78, 0x78),
            egui::Color32::from_rgb(0x2a, 0x2a, 0x2a),
        ),
        "premise: A's, B's, C's and the panel's edges"
    );

    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for (position, rating, fill) in [
        ("CF", 2, palette.rating_a_fill),
        ("SS", 1, visuals.selection.bg_fill),
        ("LWF", 0, palette.rating_c_fill),
    ] {
        for (index, node) in rating_buttons(&nodes, position).iter().enumerate() {
            let expected = if index == rating {
                edge_of(fill)
            } else {
                unheld
            };
            assert_eq!(
                strokes_over(&shapes, node.bounds().expect("bounds")).last(),
                Some(&egui::Stroke::new(1.0, expected)),
                "{position}'s {} square's edge",
                ["C", "B", "A"][index]
            );
        }
    }
    // The layout toggle's segments: Pitch held at open, the quiet grey's
    // edge; the other two the panel's. `Left` is found on Pitch's row —
    // the foot's own Left | Right toggle shares the word.
    let pitch = node_labeled(&nodes, Role::Button, "Pitch")
        .bounds()
        .expect("bounds");
    for (label, expected) in [
        ("Pitch", edge_of(palette.rating_c_fill)),
        ("Left", unheld),
        ("List", unheld),
    ] {
        let bounds = nodes
            .iter()
            .filter(|node| node.role() == Role::Button && node.label() == Some(label))
            .filter_map(accesskit::Node::bounds)
            .find(|b| (b.y0 - pitch.y0).abs() < 4.0)
            .unwrap_or_else(|| panic!("the {label} segment"));
        assert_eq!(
            strokes_over(&shapes, bounds).last(),
            Some(&egui::Stroke::new(1.0, expected)),
            "the {label} segment's edge"
        );
    }
}
