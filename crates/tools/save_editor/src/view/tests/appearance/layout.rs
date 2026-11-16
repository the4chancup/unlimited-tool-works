use super::*;

/// The `Frame::group` rect holding `point`: the smallest rect among
/// `shapes` carrying `stroke` (the group frame's own) and covering it —
/// a titled group's painted frame.
fn frame_around(
    shapes: &[egui::epaint::ClippedShape],
    stroke: egui::Stroke,
    point: egui::Pos2,
) -> egui::Rect {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) if rect.rect.contains(point) && rect.stroke == stroke => {
                Some(rect.rect)
            }
            _ => None,
        })
        .min_by(|a, b| a.area().partial_cmp(&b.area()).expect("ordered areas"))
        .expect("a group frame around the point")
}

/// The `Frame::group` rect around the group titled `title` on the drawn
/// frame (`frame_around` on the title label's point).
fn group_frame(
    window: &Headless,
    nodes: &[accesskit::Node],
    shapes: &[egui::epaint::ClippedShape],
    title: &str,
) -> egui::Rect {
    let bounds = label_bounds(nodes, title);
    frame_around(
        shapes,
        window
            .ctx
            .global_style()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke,
        egui::pos2(bounds.x0 as f32 + 2.0, bounds.y0 as f32 + 2.0),
    )
}

/// The leftmost node drawn inside `frame` under its title `title` (the
/// title's own label left out): the group's content's first left, a
/// label's or a control's alike.
fn first_content(
    nodes: &[accesskit::Node],
    frame: egui::Rect,
    title: accesskit::Rect,
) -> (&accesskit::Node, f64) {
    nodes
        .iter()
        .filter_map(|node| node.bounds().map(|bounds| (node, bounds)))
        .filter(|(_, bounds)| {
            *bounds != title
                && bounds.y0 >= title.y1 - 0.5
                && bounds.x0 >= f64::from(frame.min.x)
                && bounds.x1 <= f64::from(frame.max.x)
                && bounds.y1 <= f64::from(frame.max.y)
        })
        .map(|(node, bounds)| (node, bounds.x0))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a node under the title")
}

// Every group's content, its first column's labels and controls alike,
// stands 6 points in from its title, as the draft draws it: the title
// reads as the heading of what is under it.
#[test]
fn every_groups_content_stands_six_in_from_its_title() {
    let path = fixture(PesVersion::Pes21, "group_content_inset");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let wrong: Vec<String> = [
        "Full Player Customization",
        "Colors",
        "Strip style",
        "Physique",
        "Motion",
        "Edit flags",
        "Presentation",
    ]
    .iter()
    .filter_map(|title| {
        let frame = group_frame(&window, &nodes, &shapes, title);
        let title_bounds = label_bounds(&nodes, title);
        let (node, left) = first_content(&nodes, frame, title_bounds);
        let inset = left - title_bounds.x0;
        ((inset - 6.0).abs() > 0.5).then(|| {
            format!(
                "{title}: {:?} {:?} stands {inset} in",
                node.role(),
                node.label().or(node.value())
            )
        })
    })
    .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

// The Physique group is as wide as the draft draws it, 495 points: its
// four columns padded 6 a side, as the draft's grid cells are, so the
// labels stand 12 past the slider boxes before them.
#[test]
fn the_physique_group_is_the_drafts_width() {
    let path = fixture(PesVersion::Pes21, "physique_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let physique = group_frame(&window, &nodes, &shapes, "Physique");
    assert!(
        (physique.width() - 495.0).abs() <= 0.5,
        "the Physique group {} wide, the draft's 495",
        physique.width()
    );
}

// The Colors group's squares stand 2 points apart, as the draft's do.
#[test]
fn the_colors_squares_stand_two_apart() {
    let path = fixture(PesVersion::Pes21, "swatch_gap");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let wells = right_of_label(&nodes, "Skin", Role::ColorWell);
    let [first, second] = [0, 1].map(|at| wells[at].bounds().expect("a square's bounds"));
    let gap = second.x0 - first.x1;
    assert!(
        (gap - 2.0).abs() <= 0.25,
        "two squares {gap} apart, the draft's 2"
    );
}

// The Colors frame is the draft's 230 wide: its label column the labels'
// own width, the grid's 8-point gap to the first square, and the swatch
// cell's right padding inside the frame's right edge.
#[test]
fn the_colors_frame_is_the_drafts_230_wide() {
    let path = fixture(PesVersion::Pes21, "colors_frame_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let colors = group_frame(&window, &nodes, &shapes, "Colors");
    assert!(
        (colors.width() - 230.0).abs() <= 1.0,
        "the Colors frame {} wide, the draft's 230",
        colors.width()
    );
    let skin = label_bounds(&nodes, "Skin");
    let square = right_of_label(&nodes, "Skin", Role::ColorWell)
        .first()
        .and_then(|node| node.bounds())
        .expect("the first skin square");
    assert!(
        (square.x0 - skin.x1 - 8.0).abs() <= 1.0,
        "the first square {} after \"Skin\", the grid's 8-point gap",
        square.x0 - skin.x1
    );
}

// The Appearance screen's number boxes are the draft's `input[type=number]`
// width, 52: the motion fields', Presentation's and Copy from ID's. What
// stands beside each keeps the draft's gap: the next column's label 8 after
// a motion box, a row's note 8 after its box, and a space's width after Copy
// from ID's label, the cell the pair shares.
#[test]
fn the_appearance_number_boxes_are_the_drafts_52_wide() {
    let path = fixture(PesVersion::Pes21, "appearance_number_boxes");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let box_right_of = |label: &str| {
        right_of_label(&nodes, label, Role::SpinButton)
            .first()
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("the {label} box"))
    };
    let motion = box_right_of("Hunching (dribbling)");
    let star = box_right_of("Star rating");
    let copy = box_right_of("Copy from ID");
    for (name, bounds) in [("motion", motion), ("star", star), ("copy", copy)] {
        assert!(
            (bounds.x1 - bounds.x0 - 52.0).abs() <= 0.5,
            "the {name} box {} wide, the draft's 52",
            bounds.x1 - bounds.x0
        );
    }
    let running = label_bounds(&nodes, "Hunching (running)");
    assert!(
        (running.x0 - motion.x1 - 8.0).abs() <= 1.0,
        "the second column's label {} after the box, the grid's 8-point gap",
        running.x0 - motion.x1
    );
    let note = label_bounds(
        &nodes,
        "How likely the player is to show up as the team's featured player in the pre-match screen.",
    );
    assert!(
        (note.x0 - star.x1 - 8.0).abs() <= 1.0,
        "the star note {} after its box, the grid's 8-point gap",
        note.x0 - star.x1
    );
    let label = label_bounds(&nodes, "Copy from ID");
    assert!(
        (copy.x0 - label.x1 - 3.0).abs() <= 1.0,
        "the box {} after its label, a space's width",
        copy.x0 - label.x1
    );
}

// SE-GUI-84
#[test]
fn the_groups_stand_in_the_drafts_order() {
    let path = fixture(PesVersion::Pes21, "appearance_groups_order");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = |title: &str| group_frame(&window, &nodes, &shapes, title);
    let fpc = frame("Full Player Customization");
    let colors = frame("Colors");
    let strip = frame("Strip style");
    let physique = frame("Physique");
    let motion = frame("Motion");
    let flags = frame("Edit flags");
    let presentation = frame("Presentation");

    assert!(
        colors.min.x > fpc.max.x && (colors.min.y - fpc.min.y).abs() <= 1.0,
        "Colors at Full Player Customization's right, tops level"
    );
    assert!(
        strip.min.y > fpc.max.y && strip.min.y > colors.max.y,
        "Strip style below both"
    );
    assert!(physique.min.y > strip.max.y, "Physique below Strip style");
    assert!(
        motion.min.x > physique.max.x && (motion.min.y - physique.min.y).abs() <= 1.0,
        "Motion at Physique's right, tops level"
    );
    assert!(
        flags.min.y > motion.max.y && (flags.min.x - motion.min.x).abs() <= 1.0,
        "Edit flags under Motion, lefts level"
    );
    assert!(
        presentation.min.y > physique.max.y && presentation.min.y > flags.max.y,
        "Presentation below the row"
    );

    // The Presentation group's rows: star then playing attitude, each
    // label, box, then the group's note at the box's right on the box's
    // own row.
    for (label, note) in [
        (
            "Star rating",
            "How likely the player is to show up as the team's featured player in the pre-match screen.",
        ),
        ("Playing attitude", "Only affects cutscenes."),
    ] {
        let bounds = right_of_label(&nodes, label, Role::SpinButton)
            .first()
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("the {label} box"));
        let note_bounds = label_bounds(&nodes, note);
        assert!(
            note_bounds.x0 >= bounds.x1 - 1.0,
            "{label}'s note past its box's right"
        );
        let row_y = (bounds.y0 + bounds.y1) / 2.0;
        let note_y = (note_bounds.y0 + note_bounds.y1) / 2.0;
        assert!(
            (note_y - row_y).abs() <= 2.0,
            "{label}'s note on its box's row"
        );
    }
    assert!(
        label_bounds(&nodes, "Playing attitude").y0 > label_bounds(&nodes, "Star rating").y1,
        "the attitude row stands below the star row"
    );
}

// SE-GUI-84
#[test]
fn the_group_titles_leave_title_gap_under_them() {
    // Every group title stands TITLE_GAP (6) above the first row under
    // it, as the Stats screen's panels leave under theirs; a label's
    // bounds add its row's own centering (the 24- and 34-tall rows sit
    // their text mid-cell).
    let path = fixture(PesVersion::Pes21, "group_title_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    for (title, first, gap) in [
        ("Colors", "Skin", 7.5),
        ("Strip style", "Boots ID", 10.5),
        ("Physique", "Neck length", 15.5),
        ("Motion", "Hunching (dribbling)", 7.5),
        ("Presentation", "Star rating", 7.5),
    ] {
        let measured = label_bounds(&nodes, first).y0 - label_bounds(&nodes, title).y1;
        assert!(
            (measured - gap).abs() <= 1.0,
            "{title:?} leaves TITLE_GAP under it ({measured} vs {gap})"
        );
    }
    let flags_gap = node_labeled(&nodes, Role::CheckBox, "Face")
        .bounds()
        .expect("the Face flag's bounds")
        .y0
        - label_bounds(&nodes, "Edit flags").y1;
    assert!(
        (flags_gap - 8.0).abs() <= 1.0,
        "\"Edit flags\" leaves TITLE_GAP under it ({flags_gap} vs 8.0)"
    );
}

// SE-GUI-84
#[test]
fn the_first_row_fills_the_pane_and_ends_at_one_bottom() {
    // The draft's `.aprow.fpcrow`: Full Player Customization flexes to
    // the room Colors leaves — Colors keeps its own width at the pane's
    // right edge, SIDE_GAP between the frames — and the two frames'
    // bottoms stand level, the shorter repainted to the row's height.
    let path = fixture(PesVersion::Pes21, "first_row_bottom");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let fpc = group_frame(&window, &nodes, &shapes, "Full Player Customization");
    let colors = group_frame(&window, &nodes, &shapes, "Colors");
    assert!(
        (colors.min.x - fpc.max.x - 8.0).abs() <= 1.0,
        "SIDE_GAP between the two frames"
    );
    assert!(
        (colors.max.x - PANE_RIGHT).abs() <= 1.0,
        "the row ends at the pane's right edge ({:?} vs {PANE_RIGHT})",
        colors.max.x
    );
    assert!(
        (fpc.max.y - colors.max.y).abs() <= 0.5,
        "the two frames' bottoms stand level"
    );
}

// SE-GUI-84
#[test]
fn the_screen_settles_its_passes_once_the_colors_width_is_measured() {
    // `fpc_colors_row` asks egui to rerun the pass only when the width it
    // measured changed: once the measurement settles the screen draws in
    // one pass a frame. A guard that asked every frame instead would keep
    // every frame multipass, and egui paints its perf warning on screen
    // after three in a row — the settled screen carries none.
    let path = fixture(PesVersion::Pes21, "settled_passes");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    for frame in 0..5 {
        let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert!(
            !shapes.iter().any(|clipped| matches!(
                &clipped.shape,
                egui::Shape::Text(shape)
                    if shape.galley.job.text.contains("PERF WARNING")
            )),
            "frame {frame}: no discarded passes once the width settles"
        );
    }
}

// SE-GUI-84
#[test]
fn the_screen_shows_whole_at_the_default_window() {
    // Lock off, and lock on with the switch unticked (the lock's three
    // lines drawn): no scroll bar in the detail pane in either. The same
    // frame scrolled before the groups' layout (the section ran to over
    // a thousand points — every older test scrolled to reach its rows).
    for (setting, test) in [
        (false, "appearance_whole_unlocked"),
        (true, "appearance_whole_locked"),
    ] {
        let path = fixture(PesVersion::Pes21, test);
        let mut view = SaveView::new();
        view.open(&path);
        view.aesthetics_from_exports = setting;
        let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
        open_appearance(&mut window, &mut view);
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert!(
            !nodes.iter().any(|node| {
                node.role() == Role::ScrollBar
                    && node.bounds().is_some_and(|bounds| bounds.x0 > 300.0)
            }),
            "{test}: the detail pane has no scroll bar"
        );
    }
}

// SE-GUI-84
#[test]
fn the_strip_grid_stands_three_columns_by_five_rows_with_its_lines() {
    let path = fixture(PesVersion::Pes21, "strip_grid");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The fifteen settings, in the draft's order.
    let rows: [[&str; 3]; 5] = [
        ["Boots ID", "Sleeves", "Inners"],
        ["Gloves ID", "Socks", "Undershorts"],
        ["Wrist taping", "Wrist tape left", "Wrist tape right"],
        ["Outfielder gloves", "Gloves color", "Ankle taping"],
        ["Spectacles", "Spectacles color", "Shirttail out"],
    ];
    for (col, label) in rows[0].iter().enumerate() {
        let x0 = label_bounds(&nodes, label).x0;
        for row in &rows[1..] {
            assert!(
                (label_bounds(&nodes, row[col]).x0 - x0).abs() < 1.0,
                "{:?} shares the column's x0",
                row[col]
            );
        }
    }
    for (row, labels) in rows.iter().enumerate() {
        let y0 = label_bounds(&nodes, labels[0]).y0;
        for label in &labels[1..] {
            assert!(
                (label_bounds(&nodes, label).y0 - y0).abs() < 1.0,
                "{label:?} shares the row's top (row {row})"
            );
        }
    }
    // Every label's field starts 8 past its 104-wide label cell; the sized
    // controls are the field's one width (the checkboxes sit in a cell of
    // it, their own box narrower).
    for (row, labels) in rows.iter().enumerate() {
        for (col, label) in labels.iter().enumerate() {
            let label_bounds = label_bounds(&nodes, label);
            let control = control_right_of(&nodes, label);
            let bounds = control.bounds().expect("bounds");
            assert!(
                (bounds.x0 - label_bounds.x0 - 104.0 - 8.0).abs() <= 1.0,
                "{label:?} (row {row}, col {col}): the field 8 past the label cell"
            );
            if control.role() != Role::CheckBox {
                assert!(
                    (bounds.x1 - bounds.x0 - 118.0).abs() <= 0.5,
                    "{label:?}: the field's rect is 118 wide ({bounds:?})"
                );
            }
        }
    }

    // The lines: under the second and third rows across the three
    // columns and under the fourth across the first two, each ending
    // mid-gap half a CELL_GAP (14) past the last covered column's
    // field; and at the third column's left on rows four and five.
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let lines: Vec<[egui::Pos2; 2]> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s } if *s == stroke => Some(*points),
            _ => None,
        })
        .collect();
    let centre = |label: &str| {
        let b = label_bounds(&nodes, label);
        egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    };
    let left = label_bounds(&nodes, "Boots ID").x0 as f32;
    for (under, next, last_field) in [
        ("Gloves ID", "Wrist taping", "Inners"),
        ("Wrist taping", "Outfielder gloves", "Inners"),
        ("Outfielder gloves", "Spectacles", "Gloves color"),
    ] {
        let above = centre(under);
        let below = centre(next);
        let field_right = control_right_of(&nodes, last_field)
            .bounds()
            .expect("bounds")
            .x1 as f32;
        assert!(
            lines.iter().any(|&[a, b]| {
                a.y == b.y
                    && a.y > above.y
                    && a.y < below.y
                    && a.x.min(b.x) <= left + 4.0
                    && (a.x.max(b.x) - field_right - 14.0).abs() <= 1.0
            }),
            "the line under {under:?}'s row, ending mid-gap"
        );
    }
    let gloves_colour = control_right_of(&nodes, "Gloves color")
        .bounds()
        .expect("bounds");
    let ankle = label_bounds(&nodes, "Ankle taping");
    let row4 = centre("Outfielder gloves");
    let row5 = centre("Spectacles");
    assert!(
        lines.iter().any(|&[a, b]| {
            a.x == b.x
                && a.x > gloves_colour.x1 as f32
                && a.x < ankle.x0 as f32
                && a.y.min(b.y) <= row4.y
                && a.y.max(b.y) >= row5.y
        }),
        "the line at the third column's left on rows four and five"
    );
}

// SE-GUI-84
#[test]
fn the_edit_flags_stand_three_a_row_over_copy_from_id() {
    let path = fixture(PesVersion::Pes21, "edit_flags_grid");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let flag = |label: &str| {
        node_labeled(&nodes, Role::CheckBox, label)
            .bounds()
            .expect("bounds")
    };
    let first = ["Face", "Hair", "Physique"].map(flag);
    let second = ["Strip", "Motion", "Base copy"].map(flag);
    for bounds in &first[1..] {
        assert!(
            (bounds.y0 - first[0].y0).abs() < 1.0,
            "the first row's flags share a top"
        );
    }
    for bounds in &second[1..] {
        assert!(
            (bounds.y0 - second[0].y0).abs() < 1.0,
            "the second row's flags share a top"
        );
    }
    assert!(
        second[0].y0 - first[0].y0 >= 22.0 - 1.0,
        "the second row opens 22 under the first"
    );
    let copy = label_bounds(&nodes, "Copy from ID");
    assert!(
        copy.y0
            > first
                .iter()
                .chain(&second)
                .map(|b| b.y1)
                .fold(0.0, f64::max),
        "Copy from ID on a row under all six"
    );
}

// The Appearance screen's grids stand as the draft's do: Presentation's and
// Motion's 18-point fields 6 apart, their rows 24 apart top to top; the
// Colors rows' 14-point swatches 21 apart; and Copy from ID's box 20 under the
// second row of edit flags' squares, centred on a 22-point row of its own as
// the flags' squares are on theirs.
#[test]
fn the_appearance_grids_stand_as_the_drafts() {
    let path = fixture(PesVersion::Pes21, "appearance_grid_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let top_right_of = |label: &str, role: Role| {
        right_of_label(&nodes, label, role)
            .first()
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("a {role:?} right of {label}"))
            .y0
    };
    let strip = node_labeled(&nodes, Role::CheckBox, "Strip")
        .bounds()
        .expect("Strip's bounds");
    // A checkbox's 14-point square is centred on its widget.
    let strip_square = (strip.y0 + strip.y1) / 2.0 - 7.0;
    let gaps = [
        (
            top_right_of("Star rating", Role::SpinButton),
            top_right_of("Playing attitude", Role::SpinButton),
            24.0,
            "Playing attitude's box under Star's",
        ),
        (
            top_right_of("Hunching (dribbling)", Role::SpinButton),
            top_right_of("Arm movement (dribbling)", Role::SpinButton),
            24.0,
            "the second motion row's box under the first's",
        ),
        (
            top_right_of("Arm movement (dribbling)", Role::SpinButton),
            top_right_of("Corner kick", Role::SpinButton),
            24.0,
            "the third motion row's box under the second's",
        ),
        (
            top_right_of("Skin", Role::ColorWell),
            top_right_of("Iris", Role::ColorWell),
            21.0,
            "the Iris swatches under the Skin swatches",
        ),
        (
            strip_square,
            top_right_of("Copy from ID", Role::SpinButton),
            20.0,
            "Copy from ID's box under the Strip square",
        ),
    ];
    let wrong: Vec<String> = gaps
        .iter()
        .filter(|(upper, lower, expected, _)| (lower - upper - expected).abs() > 0.5)
        .map(|(upper, lower, expected, what)| {
            format!("{what}: {} apart, expected {expected}", lower - upper)
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

// SE-GUI-84
#[test]
fn the_physique_and_motion_flag_column_end_at_one_bottom() {
    let path = fixture(PesVersion::Pes21, "row_bottoms_level");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let physique = group_frame(&window, &nodes, &shapes, "Physique");
    let flags = group_frame(&window, &nodes, &shapes, "Edit flags");
    assert!(
        (physique.max.y - flags.max.y).abs() <= 0.5,
        "the row's two sides end at one bottom"
    );
    // The row's bottom is its frames' painted bottom, and Presentation
    // opens GROUP_GAP under it: the frame margin the row's height is
    // counted with is the frames' own, a point off moving the next
    // group's top by that much.
    let presentation = group_frame(&window, &nodes, &shapes, "Presentation");
    let row_bottom = physique.max.y.max(flags.max.y);
    assert!(
        (presentation.min.y - row_bottom - 4.0).abs() <= 0.5,
        "Presentation opens GROUP_GAP under the row ({:?} vs {row_bottom})",
        presentation.min.y
    );
}

// The Appearance screen lays out the draft's two physique pairs a row,
// each field a slider on a 34-tall row, its rail 82 points long and its
// value box at the rail's right — Neck length and Neck size share a
// row's top, Shoulder height opens the next, the seven left-column
// labels share one x0 and the second column's share another.
#[test]
fn the_physique_grid_holds_two_pairs_a_row() {
    let (mut view, _) = open_734_with("physique_two_pairs", &[], 0);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let first = label_bounds(&nodes, "Neck length").y0;
    assert!(
        (label_bounds(&nodes, "Neck size").y0 - first).abs() < 1.0,
        "Neck size shares the first row"
    );
    assert!(
        label_bounds(&nodes, "Shoulder height").y0 > first + 5.0,
        "Shoulder height opens the second row"
    );
    let left = label_bounds(&nodes, "Neck length").x0;
    for label in [
        "Shoulder height",
        "Chest",
        "Arm size",
        "Thigh",
        "Leg length",
        "Head width",
    ] {
        assert!(
            (label_bounds(&nodes, label).x0 - left).abs() < 1.0,
            "{label} shares the left column's x0"
        );
    }
    let right = label_bounds(&nodes, "Neck size").x0;
    for label in ["Neck size", "Shoulder width", "Waist", "Calf", "Head depth"] {
        assert!(
            (label_bounds(&nodes, label).x0 - right).abs() < 1.0,
            "{label} shares the second column's x0"
        );
    }
    let rail = right_of_label(&nodes, "Neck length", Role::Slider)
        .first()
        .and_then(|node| node.bounds())
        .expect("the Neck length slider");
    assert!(
        (rail.x1 - rail.x0 - 82.0).abs() <= 0.5,
        "the slider's rail is 82 points long"
    );
    let value_box = right_of_label(&nodes, "Neck length", Role::SpinButton)
        .first()
        .and_then(|node| node.bounds())
        .expect("the Neck length value box");
    assert!(
        value_box.x0 > rail.x1 && (value_box.y0 + value_box.y1 - rail.y0 - rail.y1).abs() <= 1.0,
        "the value box at the rail's right, on its line"
    );
}

// The draft's motion rows two pairs a row: the labels take two x
// positions, the hunching pair the first row, the arm pair the second,
// the kicks the third, the penalty and dribbling motions the fourth,
// the celebrations the fifth.
#[test]
fn the_motion_grid_holds_two_pairs_a_row_in_two_columns() {
    let (mut view, _) = open_734_with("motion_two_pairs", &[], 0);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let labels = [
        "Hunching (dribbling)",
        "Hunching (running)",
        "Arm movement (dribbling)",
        "Arm movement (running)",
        "Corner kick",
        "Free kick",
        "Penalty kick",
        "Dribbling motion",
        "Goal celebration 1",
        "Goal celebration 2",
    ];
    let mut xs: Vec<f64> = labels
        .iter()
        .map(|label| label_bounds(&nodes, label).x0)
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    assert_eq!(xs.len(), 2, "the motion labels take two x positions");
    // Row by row the draft's order: each row's pair shares one top.
    for (row, pair) in labels.chunks(2).enumerate() {
        let top = label_bounds(&nodes, pair[0]).y0;
        assert!(
            (label_bounds(&nodes, pair[1]).y0 - top).abs() < 1.0,
            "row {}: {:?} shares {:?}'s top",
            row + 1,
            pair[1],
            pair[0]
        );
        if row + 1 < labels.len() / 2 {
            assert!(
                label_bounds(&nodes, labels[2 * (row + 1)]).y0 > top + 5.0,
                "row {} opens below row {}",
                row + 2,
                row + 1
            );
        }
    }
}

// The same pane rule as the player view's
// (`spec/core/gui.md` "Shell layout"): on the pane the shell leaves at
// the 1280 x 780 window, the narrowest a member sees, since a narrower
// window scales the UI down, every drawn node ends inside it — on PES 17
// unlocked, and on PES 21 with the switch on.
#[test]
fn the_appearance_screen_fits_the_pane_the_shell_leaves() {
    for (version, test) in [
        (PesVersion::Pes17, "appearance_pane_17"),
        (PesVersion::Pes21, "appearance_pane_21"),
    ] {
        let path = fixture(version, test);
        let mut view = SaveView::new();
        view.open(&path);
        view.aesthetics_from_exports = true;
        view.edit_aesthetics = true;
        if version == PesVersion::Pes17 {
            view.team = TeamChoice::Team(734);
            view.selected = Some(Selection::Player(734, 0));
        }
        let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
        open_appearance(&mut window, &mut view);
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        for node in &nodes {
            if let Some(bounds) = node.bounds() {
                assert!(
                    bounds.x1 <= f64::from(DEFAULT_PANE.x),
                    "{version:?}: {:?} {} clips the pane's right edge ({bounds:?})",
                    node.role(),
                    node.label().or(node.value()).unwrap_or("?")
                );
            }
        }
    }
}

/// One `tool.view` frame's nodes (the settings-menu path's tests draw the
/// real `Tool`, which refreshes the view's settings each frame).
fn tool_frame(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
) -> Vec<accesskit::Node> {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, ctx));
    nodes
}

/// `open_menu` for `tool.view` frames: the toolbar menu `menu`'s popup open
/// (skipped when a previous call left `item` drawn), then the frame's nodes.
fn open_tool_menu(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
    menu: &str,
    item: &str,
) -> Vec<accesskit::Node> {
    let shows_item = |nodes: &[accesskit::Node]| {
        nodes.iter().any(|node| {
            node.role() == Role::Button && node.label().is_some_and(|l| l.starts_with(item))
        })
    };
    let nodes = tool_frame(window, tool, ctx);
    if shows_item(&nodes) {
        return nodes;
    }
    // The pointer onto the button first: while another menu is open that
    // opens this one, and a click would close it again (`open_menu`).
    let button = centre(menu_button(&nodes, menu));
    window.frame(vec![egui::Event::PointerMoved(button)], |ui| {
        tool.view(ui, ctx)
    });
    let nodes = tool_frame(window, tool, ctx);
    if shows_item(&nodes) {
        return nodes;
    }
    press_release(window, centre(menu_button(&nodes, menu)), |ui| {
        tool.view(ui, ctx)
    });
    tool_frame(window, tool, ctx)
}

// SE-GUI-35
#[test]
fn the_setting_off_draws_no_lock_and_the_setting_on_locks_the_paths() {
    // The default, a cup whose looks live in the save: no ownership line
    // and no switch, every path SE-GUI-22 names enabled, and an
    // Appearance edit landing like an Abilities one.
    let path = fixture(PesVersion::Pes21, "aesthetics_setting_default");
    let (ctx, _events_rx, _requests_rx) =
        tool_context(path.parent().unwrap(), studio_core::Settings::default());
    let mut tool = crate::Tool::new();
    tool.view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 1000.0);
    open_appearance(&mut window, &mut tool.view);

    let nodes = tool_frame(&mut window, &mut tool, &ctx);
    assert!(
        !nodes.iter().any(|node| {
            node.value()
                .is_some_and(|value| value.starts_with("The team's export owns"))
        }),
        "no ownership line under the default"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::CheckBox && node.label() == Some("Edit aesthetics anyway")
        }),
        "no switch under the default"
    );
    let chest = right_of_label(&nodes, "Chest", Role::SpinButton)
        .first()
        .copied()
        .expect("the Chest control");
    assert!(!chest.is_disabled(), "the section's controls are enabled");

    let nodes = open_tool_menu(&mut window, &mut tool, &ctx, "Player", "Set all stats");
    assert!(
        !menu_item(&nodes, "Hide default body (Enable FPC)").is_disabled(),
        "the FPC entry is enabled"
    );
    let nodes = open_tool_menu(&mut window, &mut tool, &ctx, "Team", "Set edited flag");
    for label in [
        "Set boots and gloves IDs for everyone",
        "Clear visual flags",
        "Hide default body (Enable FPC)",
        "Show default body (Disable FPC)",
    ] {
        assert!(
            !menu_item(&nodes, label).is_disabled(),
            "{label} is enabled"
        );
    }
    let nodes = open_tool_menu(&mut window, &mut tool, &ctx, "Savefile", "Fix database");
    assert!(
        !menu_item(&nodes, "Fix database").is_disabled(),
        "Fix database is enabled"
    );

    // The edit lands: premise Chest stores 7 (shown 0), typed 5 stores 12.
    // The row sits below the fold: scroll until the clip covers it.
    assert_eq!(
        tool.view
            .session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .chest,
        7,
        "premise: Chest stores 7 (shown 0)"
    );
    let nodes = scroll_into_view(&mut window, &mut tool.view, |node| {
        node.role() == Role::Label && node.value() == Some("Chest")
    });
    let chest = right_of_label(&nodes, "Chest", Role::SpinButton)
        .first()
        .copied()
        .expect("the Chest control");
    press_release(&mut window, centre(chest), |ui| tool.view(ui, &ctx));
    window.frame(vec![egui::Event::Text("5".to_owned())], |ui| {
        tool.view(ui, &ctx)
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| tool.view(ui, &ctx),
    );
    assert_eq!(
        tool.view
            .session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .chest,
        12,
        "typed 5 stores 12"
    );

    // The setting switched on in the settings menu: the next frame draws
    // the switch, off, and the paths disabled.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.settings_view(ui, &ctx));
    let checkbox = right_of_label(&nodes, "Aesthetics from exports", Role::CheckBox)
        .first()
        .copied()
        .expect("the setting's checkbox");
    assert_eq!(
        checkbox.toggled(),
        Some(accesskit::Toggled::False),
        "off by default"
    );
    // The checkbox's hover text is the setting's sentence.
    let at = centre(checkbox);
    window.frame(vec![egui::Event::PointerMoved(egui::Pos2::ZERO)], |ui| {
        tool.settings_view(ui, &ctx)
    });
    window.frame(vec![egui::Event::PointerMoved(at)], |ui| {
        tool.settings_view(ui, &ctx)
    });
    let mut tooltip = None;
    for _ in 0..90 {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.settings_view(ui, &ctx));
        if let Some(value) = nodes
            .iter()
            .filter_map(|node| node.value())
            .find(|value| value.contains("the cup's looks are compiled from exports"))
        {
            tooltip = Some(value.to_owned());
            break;
        }
    }
    assert_eq!(
        tooltip.as_deref(),
        Some(
            "On when the cup's looks are compiled from exports: the save's aesthetics are \
             then read-only, behind Edit aesthetics anyway on the Appearance screen."
        ),
        "the checkbox's hover text"
    );
    press_release(&mut window, at, |ui| tool.settings_view(ui, &ctx));
    let nodes = tool_frame(&mut window, &mut tool, &ctx);
    assert_eq!(
        edit_anyway(&nodes).toggled(),
        Some(accesskit::Toggled::False),
        "the switch drawn, off"
    );
    assert!(
        right_of_label(&nodes, "Chest", Role::SpinButton)[0].is_disabled(),
        "locked once the setting is on"
    );
    let nodes = open_tool_menu(&mut window, &mut tool, &ctx, "Team", "Set edited flag");
    assert!(
        menu_item(&nodes, "Clear visual flags").is_disabled(),
        "the lock's paths are disabled"
    );
}
