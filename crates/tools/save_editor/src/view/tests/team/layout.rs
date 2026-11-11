use super::*;

// The drafts' window on a version without kit slots: the short dialog is the
// window's 360 points wide, OK at its right inside the stroke and the
// 12-point margin.
#[test]
fn the_kit_slots_dialog_is_360_wide_with_ok_at_the_right() {
    let path = fixture(PesVersion::Pes21, "kit_slots_window");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Kit slots\u{2026}"),
        &mut view,
    );
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "(this version does not store kit slots)"),
        "premise: PES 21 stores no kit slots"
    );
    let outer = window
        .ctx
        .memory(|memory| memory.area_rect(egui::Id::unique("save_editor_kit_slots")))
        .expect("the dialog's modal has an area");
    let ok = node_labeled(&nodes, Role::Button, "OK")
        .bounds()
        .expect("OK has bounds");
    let width = outer.width();
    assert!(
        (width - 360.0).abs() <= 0.5,
        "the dialog is 360 wide, not {width}"
    );
    let right = f64::from(outer.right()) - ok.x1;
    assert!(
        (right - 13.0).abs() <= 0.5,
        "OK's right is 13 inside the dialog's, not {right}"
    );
}

// The bar's name field gets its 140 points (the draft's `#team-name`)
// from `add_sized`: the field keeps its own width rather than the wrapped
// row's offer, measured on the frame the team view first draws (the
// click's release frame), not a later one where a collapsed first frame
// would already be past.
#[test]
fn the_team_name_field_gets_its_full_width_on_the_first_frame() {
    let path = fixture(PesVersion::Pes21, "team_name_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The release frame is the first one that draws the team view.
    let nodes = click_at(&mut window, centre(team_row(&nodes, &view, 701)), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "the click selected team 701's row"
    );
    let name = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some("/3/"))
        .expect("the team's Name field");
    let bounds = name.bounds().expect("bounds");
    assert!(
        (bounds.x1 - bounds.x0 - 140.0).abs() <= 1.5,
        "the Name field draws at its 140 points on the first frame, got {bounds:?}"
    );
}

// The team view's title is `<id> <name>` in the body text's size, its
// name in the strong weight, not a heading's size.
#[test]
fn the_title_is_the_teams_name_in_the_bodys_strong_weight() {
    let path = fixture(PesVersion::Pes21, "team_title_body");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (body, strong) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (
            egui::TextStyle::Body.resolve(&style),
            style.visuals.strong_text_color(),
        )
    };
    let title = label_shape(&nodes, &shapes, "701 /3/");
    for section in &title.galley.job.sections {
        assert_eq!(
            section.format.font_id.size, body.size,
            "the title's font is the body's"
        );
    }
    assert_eq!(
        title
            .galley
            .job
            .sections
            .last()
            .expect("the name's run")
            .format
            .color,
        strong,
        "the title's plain name run in the strong colour"
    );
}

// Each team colour's `#RRGGBB` beside its swatch is in the weak grey, as the
// draft draws it, and in the monospace font.
#[test]
fn the_colour_codes_are_in_the_weak_grey() {
    let path = fixture(PesVersion::Pes21, "team_colour_codes_weak");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (weak, monospace) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (
            style.visuals.weak_text_color(),
            egui::TextStyle::Monospace.resolve(&style),
        )
    };
    for label in ["Col. 1", "Col. 2"] {
        let hex = right_of_label(&nodes, label, Role::Label)
            .into_iter()
            .find(|node| node.value().is_some_and(|v| v.starts_with('#')))
            .unwrap_or_else(|| panic!("{label}'s hex"))
            .value()
            .expect("the hex's text")
            .to_owned();
        assert_eq!(
            painted_label_color(&nodes, &shapes, &hex),
            weak,
            "{label}'s {hex} in the weak grey"
        );
        let painted = label_shape(&nodes, &shapes, &hex);
        for section in &painted.galley.job.sections {
            assert_eq!(section.format.font_id, monospace, "{label}'s {hex} font");
        }
    }
}

// At the 1280 x 780 default pane the team view's fields, the ids and the
// `Kit slots…` button share one row, the name box is the draft's 140
// points and the colours are labelled `Col. 1`/`Col. 2`.
#[test]
fn the_fields_ids_and_kit_slots_button_share_one_row_at_the_default_pane() {
    let path = fixture(PesVersion::Pes21, "team_header_row");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let row_y = label_bounds(&nodes, "Short").y0;
    for label in ["Name", "Col. 1", "Col. 2", "Manager ID", "Stadium ID"] {
        let bounds = nodes
            .iter()
            .filter(|node| node.role() == Role::Label && node.value() == Some(label))
            .filter_map(|node| node.bounds())
            .min_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"))
            .unwrap_or_else(|| panic!("no {label:?} label"));
        assert!(
            (bounds.y0 - row_y).abs() < 4.0,
            "{label} shares the fields' row ({bounds:?} vs {row_y})"
        );
    }
    let button = node_labeled(&nodes, Role::Button, "Kit slots\u{2026}")
        .bounds()
        .expect("the Kit slots button's bounds");
    assert!(
        (button.y0 - row_y).abs() < 4.0,
        "the Kit slots button shares the row ({button:?} vs {row_y})"
    );
    let name = bar_name_field(&nodes).bounds().expect("bounds");
    assert!(
        (name.x1 - name.x0 - 140.0).abs() <= 1.5,
        "the name box is the draft's 140 points ({name:?})"
    );
}

// The row's controls stand where the draft's do, measured from the Name
// box's left edge (the draft at 1280 x 780, PES 21 save, team 701): the
// units 8 apart, egui's item spacing, a colour's or an id's parts a space
// apart, the ids' cells padded 4 on each side.
#[test]
fn the_rows_controls_stand_where_the_drafts_do_from_the_name_box() {
    let path = fixture(PesVersion::Pes21, "team_row_offsets");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let name = bar_name_field(&nodes).bounds().expect("bounds").x0;
    let left_of = |label: &str, role: Role| {
        right_of_label(&nodes, label, role)
            .first()
            .copied()
            .unwrap_or_else(|| panic!("no {role:?} right of {label:?}"))
            .bounds()
            .expect("bounds")
            .x0
    };
    let kit_slots = node_labeled(&nodes, Role::Button, crate::view::team::KIT_SLOTS_LABEL)
        .bounds()
        .expect("bounds")
        .x0;
    let label = |text: &str| label_bounds(&nodes, text).x0;
    let edges = [
        ("the Short label", label("Short"), 148.0),
        ("the Short box", left_of("Short", Role::TextInput), 186.6),
        ("Col. 1's swatch", left_of("Col. 1", Role::ColorWell), 270.0),
        ("Col. 2's swatch", left_of("Col. 2", Role::ColorWell), 389.2),
        ("the Manager ID label", label("Manager ID"), 476.8),
        (
            "the Manager ID box",
            left_of("Manager ID", Role::SpinButton),
            544.9,
        ),
        ("the Stadium ID label", label("Stadium ID"), 598.9),
        (
            "the Stadium ID box",
            left_of("Stadium ID", Role::SpinButton),
            663.7,
        ),
        ("the Kit slots button", kit_slots, 721.7),
    ];
    for (control, left, draft) in edges {
        assert!(
            (left - name - draft).abs() <= 1.0,
            "{control} stands {draft} right of the Name box, got {}",
            left - name
        );
    }
}

// The id cells' padding, 4 on each side, stands them apart exactly: the
// Stadium ID label starts 8 past the Manager ID box (the two paddings, no
// row gap between the cells) and the Manager ID label 12 past Col. 2's hex
// (the row's gap and Manager ID's padding).
#[test]
fn the_id_cells_stand_their_paddings_apart() {
    let path = fixture(PesVersion::Pes21, "team_id_cell_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let colour_hex_right = right_of_label(&nodes, "Col. 2", Role::Label)
        .into_iter()
        .find(|node| node.value().is_some_and(|v| v.starts_with('#')))
        .expect("Col. 2's hex")
        .bounds()
        .expect("bounds")
        .x1;
    let manager_box_right = right_of_label(&nodes, "Manager ID", Role::SpinButton)
        .first()
        .copied()
        .expect("the manager control")
        .bounds()
        .expect("bounds")
        .x1;
    let gaps = [
        (
            "Col. 2's hex to the Manager ID label",
            label_bounds(&nodes, "Manager ID").x0 - colour_hex_right,
            12.0,
        ),
        (
            "the Manager ID box to the Stadium ID label",
            label_bounds(&nodes, "Stadium ID").x0 - manager_box_right,
            8.0,
        ),
    ];
    for (between, gap, expected) in gaps {
        assert!(
            (gap - expected).abs() <= 0.01,
            "{between} is {expected}, got {gap}"
        );
    }
}

/// The left edges of the team row's Short box, colour swatches, id
/// controls and `Kit slots…` button, from the Name box's, on `version`'s
/// fixture: the id controls are the first control right of their labels,
/// the number box or its disabled stand-in.
fn row_edges(version: PesVersion, test: &str) -> [f64; 6] {
    let path = fixture(version, test);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name = bar_name_field(&nodes).bounds().expect("bounds").x0;
    let left = |node: &accesskit::Node| node.bounds().expect("bounds").x0 - name;
    [
        left(control_right_of(&nodes, "Short")),
        left(control_right_of(&nodes, "Col. 1")),
        left(control_right_of(&nodes, "Col. 2")),
        left(control_right_of(&nodes, "Manager ID")),
        left(control_right_of(&nodes, "Stadium ID")),
        left(node_labeled(
            &nodes,
            Role::Button,
            crate::view::team::KIT_SLOTS_LABEL,
        )),
    ]
}

// On PES 16, whose save stores no manager or stadium ID, the ids' disabled
// stand-ins take the number boxes' width: the row's controls stand where
// they do on PES 21.
#[test]
fn the_rows_controls_stand_still_on_a_version_without_the_ids() {
    const CONTROLS: [&str; 6] = [
        "the Short box",
        "Col. 1's swatch",
        "Col. 2's swatch",
        "the Manager ID control",
        "the Stadium ID control",
        "the Kit slots button",
    ];
    let stored = row_edges(PesVersion::Pes21, "team_row_edges_21");
    let gated = row_edges(PesVersion::Pes16, "team_row_edges_16");
    for ((control, gated), stored) in CONTROLS.iter().zip(gated).zip(stored) {
        assert!(
            (gated - stored).abs() <= 1.0,
            "{control} stands {gated} on PES 16 against {stored} on PES 21"
        );
    }
}

// The ids share the fields' row because the Tactics editor below needs
// the height: at the default pane the view's lowest drawn stroke (the
// lists' and Automatic settings' frame bottoms) paints the draft's last
// row, pixel row 741 of the 1280 x 780 client, whose y the pane shares.
#[test]
fn the_team_view_fits_the_default_pane() {
    let path = fixture(PesVersion::Pes21, "team_view_fit");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    // Three frames: the scroll's claimed content size lands a frame late.
    let mut shapes = Vec::new();
    for _ in 0..3 {
        let (_, _, frame_shapes) =
            window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        shapes = frame_shapes;
    }
    let left = f64::from(pane_frame(&shapes, window.screen).min.x);
    let bottom = shapes
        .iter()
        .map(|clipped| clipped.shape.visual_bounding_rect())
        .filter(|rect| {
            // Inside the detail pane, and not its own full-height frame.
            f64::from(rect.min.x) >= left - 1.0 && rect.height() < window.screen.y * 0.75
        })
        .map(|rect| f64::from(rect.max.y))
        .fold(0.0, f64::max);
    // A 1-point stroke paints the row above its bounding bottom.
    let last_row = bottom - 1.0;
    assert!(
        (last_row - 741.0).abs() <= 1.0,
        "the lowest stroke paints the draft's last row, 741 ({last_row})"
    );
}

/// The team view's fields' row and the line under it, from one frame of
/// team 701's view at the default pane: the bottom of the row's tallest
/// control, the line's painted row (the top of its stroke, as the draft's
/// `.sep` border covers one pixel row) and the line's left end.
fn fields_line(test: &str) -> (Vec<accesskit::Node>, f64, f64, f64) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let name = bar_name_field(&nodes).bounds().expect("bounds");
    let on_row = |b: &accesskit::Rect| b.y0 < name.y1 && b.y1 > name.y0;
    let row_bottom = nodes
        .iter()
        .filter(|node| {
            matches!(
                node.role(),
                Role::TextInput | Role::ColorWell | Role::SpinButton | Role::Button
            )
        })
        .filter_map(accesskit::Node::bounds)
        .filter(|b| on_row(b) && b.x0 >= name.x0)
        .map(|b| b.y1)
        .fold(name.y1, f64::max);
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let (line, line_left) = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && f64::from(points[0].y) > row_bottom
                    && points[0].x.min(points[1].x) <= name.x0 as f32 =>
            {
                Some((
                    f64::from(points[0].y),
                    f64::from(points[0].x.min(points[1].x)),
                ))
            }
            _ => None,
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered ys"))
        .expect("a stroke line under the fields' row");
    let painted_row = line - f64::from(stroke.width) / 2.0;
    (nodes, row_bottom, painted_row, line_left)
}

// The draft's `.sep` under the fields' row: its painted row 11 points below
// the row's tallest control (the row's 3-point margin and the line's 8),
// the Tactics' first control 11.5 below that row (the line's own point, its
// 8-point margin and the bar's 2.5 over the Fluid formation checkbox).
#[test]
fn the_line_under_the_fields_stands_11_from_the_row_and_11_5_from_the_tactics() {
    let (nodes, row_bottom, line, line_left) = fields_line("team_separator_gaps");
    // The view's nodes only (the line spans the view): the player list's
    // rows left of it run past the line too.
    let tactics_top = nodes
        .iter()
        .filter_map(accesskit::Node::bounds)
        .filter(|b| b.y0 > line && b.x0 >= line_left)
        .map(|b| b.y0)
        .fold(f64::MAX, f64::min);
    assert!(
        (line - row_bottom - 11.0).abs() <= 0.5,
        "the line's row stands 11 below the row's tallest control ({line} vs {row_bottom})"
    );
    assert!(
        (tactics_top - line - 11.5).abs() <= 0.5,
        "the Tactics' first control stands 11.5 below the line's row ({tactics_top} vs {line})"
    );
}

// The team view's pane ends where the draft's `.detail` content does, at
// the view's right edge, 8 points short of the window's: the line under
// the fields, which spans the pane, ends there, as the draft's `.sep`
// does at 1272.
#[test]
fn the_team_views_line_ends_8_from_the_windows_edge() {
    let path = fixture(PesVersion::Pes21, "team_pane_right");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let line_right = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && (points[0].x - points[1].x).abs() > window.screen.x * 0.5 =>
            {
                Some(f64::from(points[0].x.max(points[1].x)))
            }
            _ => None,
        })
        .expect("the line under the fields, across the pane");
    assert!(
        (line_right - f64::from(PANE_RIGHT)).abs() <= 1.0,
        "the line ends at {PANE_RIGHT}, the draft's 1272 ({line_right})"
    );
}

// The Tactics bars' choices (the draft's `.sel`s, 17 tall, centred in the
// 23-point `.tacbar` 8 under the line) stand 12 under the line's painted
// row and end 29 under it: the draft's 98 to 115 under its row 86.
#[test]
fn the_tactics_bars_choices_stand_12_under_the_line() {
    let (nodes, _, line, _) = fields_line("team_bar_under_line");
    for label in [
        "1",
        "2",
        "3",
        "Kick-off",
        "In-possession",
        "Out-of-possession",
    ] {
        let choice = node_labeled(&nodes, Role::Button, label)
            .bounds()
            .expect("bounds");
        assert!(
            (choice.y0 - line - 12.0).abs() <= 1.0,
            "{label} starts 12 under the line's row ({} vs {line})",
            choice.y0
        );
        assert!(
            (choice.y1 - line - 29.0).abs() <= 1.0,
            "{label} ends 29 under the line's row ({} vs {line})",
            choice.y1
        );
    }
}

// The fields' row's boxes stand 7 under the title's line box: the draft's
// title keeps 4 points under it and the row its own 3-point margin round
// its controls (its 57 under the title's 50).
#[test]
fn the_fields_row_stands_7_under_the_title() {
    let path = fixture(PesVersion::Pes21, "team_row_under_title");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let title = label_bounds(&nodes, "701 /3/");
    let name = bar_name_field(&nodes).bounds().expect("bounds");
    assert!(
        (name.y0 - title.y1 - 7.0).abs() <= 1.0,
        "the Name box stands 7 under the title ({} vs {})",
        name.y0,
        title.y1
    );
}
