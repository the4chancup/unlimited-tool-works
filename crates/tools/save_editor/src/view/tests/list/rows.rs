use super::*;

// A click in the gap between two rows of the team dropdown picks the
// nearer row, as a native list's rows meet (the popup's `Area` is the
// widget a click landing between rows would otherwise hit). A touch
// event first, so the scroll background's drag sense is on as on a
// machine with a touch screen.
// ST-COMBO-01
#[test]
fn a_gap_click_in_the_team_dropdown_picks_the_nearer_team() {
    let (mut view, mut window) = open_734_with("combo_gap_team", &[73401], 183);
    window.frame(
        vec![egui::Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(1),
            phase: egui::TouchPhase::Start,
            pos: egui::pos2(250.0, 50.0),
            force: None,
        }],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let (a_name, b_name, b_id) = {
        let file = view.session.as_ref().expect("the save opened").file();
        let (a, b) = (&file.teams()[3], &file.teams()[4]);
        (
            display_name(&a.name).to_owned(),
            display_name(&b.name).to_owned(),
            b.id,
        )
    };
    let nodes = draw(&mut window, &mut view);
    let combo = nodes
        .iter()
        .find(|node| node.role() == Role::ComboBox && node.value() == Some("/lit/"))
        .expect("the team selector")
        .clone();
    click(&mut window, &combo, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds_of = |label: &str| {
        pairs
            .iter()
            .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label))
            .and_then(|(_, node)| node.bounds())
            .unwrap_or_else(|| panic!("no popup row {label:?}"))
    };
    let (a_bounds, b_bounds) = (bounds_of(&a_name), bounds_of(&b_name));
    assert!(
        a_bounds.y1 < b_bounds.y0 && b_bounds.y0 - a_bounds.y1 < 6.0,
        "premise: two adjacent rows ({a_bounds:?}, {b_bounds:?})"
    );
    press_release(
        &mut window,
        egui::pos2(
            ((b_bounds.x0 + b_bounds.x1) / 2.0) as f32,
            (b_bounds.y0 - 0.5) as f32,
        ),
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.team,
        TeamChoice::Team(b_id),
        "the nearer (lower) row picked"
    );
}

// The team row's own paint, the draft's `trow`: the id in the weak
// colour at the row's left padding, the colours 1-2 as swatches 3 apart,
// the name in the strong colour after a further item spacing, each
// centred on the row — over the faint fill that sets it apart from a
// player row, the selection's or the hover's fill over that.
// SE-GUI-37
#[test]
fn the_team_row_paints_its_parts_over_its_fills() {
    /// Whether a `Shape::Rect` of `fill` covers `rect`'s centre and spans
    /// its width (the row's own fill, not a swatch's or the panel's).
    fn covers_row(
        shapes: &[egui::epaint::ClippedShape],
        rect: accesskit::Rect,
        fill: egui::Color32,
    ) -> bool {
        let centre = egui::pos2(
            ((rect.x0 + rect.x1) / 2.0) as f32,
            ((rect.y0 + rect.y1) / 2.0) as f32,
        );
        shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(shape) if shape.fill == fill
                && shape.rect.contains(centre)
                && f64::from(shape.rect.width()) >= rect.x1 - rect.x0 - 1.0)
        })
    }

    let path = fixture(PesVersion::Pes21, "team_row_paint");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    // The stored colours the swatches show, read as a premise.
    let colors = team_entry(&view, 701)
        .colors
        .expect("PES 21 stores team colours");
    let mut window = Headless::new();
    let (spacing, visuals) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (style.spacing.clone(), style.visuals.clone())
    };
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    let centre_y = (row.y0 + row.y1) / 2.0;
    let padding = f64::from(spacing.button_padding.x);
    let item_spacing = f64::from(spacing.item_spacing.x);

    // The id's galley: its left at the row's left plus the button padding,
    // centred on the row, in the weak colour.
    let id_shape = text_in_rect(&shapes, "701", row);
    assert!(
        (f64::from(id_shape.pos.x) - (row.x0 + padding)).abs() <= 0.5,
        "the id starts at the row's left plus the padding: {row:?} vs {:?}",
        id_shape.pos
    );
    assert!(
        (f64::from(id_shape.pos.y) + f64::from(id_shape.galley.size().y) / 2.0 - centre_y).abs()
            <= 0.5,
        "the id centres on the row"
    );
    assert_eq!(
        id_shape.galley.job.sections.first().map(|s| s.format.color),
        Some(visuals.weak_text_color()),
        "the id in the weak colour"
    );

    // The two swatches, each 10 x 10 centred on the row: the first an item
    // spacing after the id, the second 3 after the first.
    let mut swatches: Vec<(egui::Rect, egui::Color32)> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if (f64::from(shape.rect.width()) - 10.0).abs() < 0.5
                    && (f64::from(shape.rect.height()) - 10.0).abs() < 0.5
                    && (f64::from(shape.rect.center().y) - centre_y).abs() < 1.0
                    && f64::from(shape.rect.min.x) >= row.x0
                    && f64::from(shape.rect.max.x) <= row.x1 =>
            {
                Some((shape.rect, shape.fill))
            }
            _ => None,
        })
        .collect();
    swatches.sort_by(|a, b| a.0.min.x.total_cmp(&b.0.min.x));
    assert_eq!(swatches.len(), 2, "the two colour swatches");
    let id_end = f64::from(id_shape.pos.x + id_shape.galley.size().x);
    assert!(
        (f64::from(swatches[0].0.min.x) - (id_end + item_spacing)).abs() <= 0.5,
        "the first swatch an item spacing after the id"
    );
    assert!(
        (f64::from(swatches[1].0.min.x) - (f64::from(swatches[0].0.max.x) + 3.0)).abs() <= 0.5,
        "the second swatch 3 after the first"
    );
    for (index, (_rect, fill)) in swatches.iter().enumerate() {
        let color = colors[index];
        let expected = egui::Color32::from_rgb(
            crate::view::team::channel_to_show(color.red),
            crate::view::team::channel_to_show(color.green),
            crate::view::team::channel_to_show(color.blue),
        );
        assert_eq!(*fill, expected, "colour {}'s swatch", index + 1);
    }

    // The name's galley an item spacing after the second swatch, centred
    // on the row, the plain runs in the strong colour.
    let name_shape = text_in_rect(&shapes, "/3/", row);
    assert!(
        (f64::from(name_shape.pos.x) - (f64::from(swatches[1].0.max.x) + 3.0 + item_spacing)).abs()
            <= 0.5,
        "the name follows the second swatch"
    );
    assert!(
        (f64::from(name_shape.pos.y) + f64::from(name_shape.galley.size().y) / 2.0 - centre_y)
            .abs()
            <= 0.5,
        "the name centres on the row"
    );
    assert_eq!(
        name_shape.fallback_color,
        visuals.strong_text_color(),
        "the name in the strong colour"
    );

    // Unselected under no pointer the row paints the faint fill that sets
    // it apart from a player row; hovered the hover's, selected the
    // selection's.
    assert!(
        covers_row(&shapes, row, visuals.faint_bg_color),
        "the faint fill under the unselected row"
    );
    let (_, _, shapes) = window.frame(
        vec![egui::Event::PointerMoved(egui::pos2(
            ((row.x0 + row.x1) / 2.0) as f32,
            centre_y as f32,
        ))],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(
        covers_row(&shapes, row, visuals.widgets.hovered.bg_fill),
        "the hover fill under the hovered row"
    );
    assert!(
        !covers_row(&shapes, row, visuals.faint_bg_color),
        "no faint fill under the hovered row"
    );
    // Selected with the pointer gone (a frame before: the frame that
    // receives `PointerGone` still reports the row hovered), so the
    // selection alone holds the faint fill back.
    window.frame(vec![egui::Event::PointerGone], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    view.selected = Some(Selection::Team(701));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    assert!(
        covers_row(&shapes, row, visuals.selection.bg_fill),
        "the selection fill under the selected row"
    );
    assert!(
        !covers_row(&shapes, row, visuals.faint_bg_color),
        "no faint fill under the selected row"
    );

    // Unselected, the pointer still gone, focused by the keyboard: no
    // faint fill either.
    view.selected = None;
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let label = format!("701 {}", display_name(&team_entry(&view, 701).name));
    let (row_id, _) = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label.as_str()))
        .expect("the team row's node");
    action_focus(&mut window, &mut view, *row_id);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !covers_row(&shapes, row, visuals.faint_bg_color),
        "no faint fill under the focused row"
    );
}

// The same row, the name too long for the side panel: its galley ends
// inside the row's right minus the padding rather than past it.
#[test]
fn a_team_name_too_long_truncates_inside_the_row() {
    let path = fixture(PesVersion::Pes21, "team_row_long_name");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    // The version's own field length in bytes, read from the schema —
    // long enough that the name never fits the side panel.
    let name = "A".repeat(
        usize::try_from(
            pes_savefile::schema::schema_for(PesVersion::Pes21)
                .team
                .texts
                .iter()
                .find(|spec| spec.text == pes_savefile::schema::fields::TeamText::Name)
                .expect("every version stores the team name")
                .len,
        )
        .expect("a text field's length fits usize"),
    );
    file.team_mut(701).unwrap().name = name.clone();
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let padding = f64::from(
        window
            .ctx
            .style_of(window.ctx.theme())
            .spacing
            .button_padding
            .x,
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    let name_shape = text_in_rect(&shapes, &name, row);
    assert!(name_shape.galley.elided, "the galley is truncated");
    let end = f64::from(name_shape.pos.x + name_shape.galley.size().x);
    assert!(
        end <= row.x1 - padding + 0.5 && end >= row.x1 - padding - 10.0,
        "the name's galley ends on the row's right minus the padding ({end} vs {})",
        row.x1 - padding
    );
}

// The whole player row is one click target, as the team row is: every
// player row spans the team row's width, the selected row's selection
// fill covers it, and a click past a short label's text still selects
// him.
#[test]
fn every_player_row_spans_the_team_rows_width_and_selects_at_its_end() {
    let path = fixture(PesVersion::Pes21, "row_right_click");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let team = team_row(&nodes, &view, 701)
        .bounds()
        .expect("team row bounds");

    // Every player row's bounds span the team row's x range.
    let rows: Vec<&accesskit::Node> = nodes.iter().filter(|node| player_row(node)).collect();
    assert!(!rows.is_empty(), "player rows are drawn");
    for row in &rows {
        let bounds = row.bounds().expect("row bounds");
        assert!(
            (bounds.x0 - team.x0).abs() <= 1.0 && (bounds.x1 - team.x1).abs() <= 1.0,
            "a player row spans the team row's x range ({bounds:?} vs {team:?})"
        );
    }

    // The selected row's fill covers that width (a content-wide button
    // leaves the row's right half unpainted).
    let selected = rows
        .iter()
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .expect("the selected row")
        .bounds()
        .expect("bounds");
    let centre = egui::pos2(
        ((selected.x0 + selected.x1) / 2.0) as f32,
        ((selected.y0 + selected.y1) / 2.0) as f32,
    );
    assert!(
        shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape) if shape.fill == visuals.selection.bg_fill
                && shape.rect.contains(centre)
                && f64::from(shape.rect.width()) >= team.x1 - team.x0 - 1.0)),
        "the selection fill covers the row's width"
    );

    // A click at the row's right end, past a short label's text, selects
    // him (the list's scroll bar stands outside the rows' frame).
    view.selected = Some(Selection::Player(701, 3));
    let at = egui::pos2((team.x1 - 2.0) as f32, centre.y);
    click_at(&mut window, at, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "the click selected the row"
    );
}

// SE-GUI-37's team-row dot: a team's own unsaved changes mark its row's
// right end — after a short-name edit the `●` paints there in the
// warning colour, right of the painted name — and no player row's label
// carries one (the dot is no part of the row's AccessKit label).
// SE-GUI-37
#[test]
fn the_team_rows_dot_paints_at_the_rows_right_end() {
    let path = fixture(PesVersion::Pes21, "team_row_dot");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let mut team = team_entry(&view, 701);
    team.short_name = "ZED".to_owned();
    view.session.as_mut().unwrap().edit_team(1, team);

    let padding = f64::from(
        window
            .ctx
            .style_of(window.ctx.theme())
            .spacing
            .button_padding
            .x,
    );
    let warn = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .warn_fg_color;
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = team_row(&nodes, &view, 701).bounds().expect("bounds");
    let name = text_in_rect(&shapes, "/3/", bounds);
    let dot = dot_in_row(&shapes, bounds);
    assert_eq!(galley_color(dot), warn, "the dot in the warning colour");
    let dot_right = f64::from(dot.pos.x + dot.galley.rect.width());
    assert!(
        (dot_right - (bounds.x1 - padding)).abs() <= 0.5,
        "the dot's right edge at the row's right less the padding ({dot_right} vs {})",
        bounds.x1 - padding
    );
    assert!(
        f64::from(dot.pos.x) >= f64::from(name.pos.x + name.galley.rect.width()),
        "the dot sits right of the painted name"
    );
    let row_centre = (bounds.y0 + bounds.y1) / 2.0;
    let dot_centre = f64::from(dot.pos.y + dot.galley.size().y / 2.0);
    assert!(
        (dot_centre - row_centre).abs() <= 0.5,
        "the dot centres on the row ({dot_centre} vs {row_centre})"
    );
    assert!(
        row_labels(&nodes)
            .iter()
            .all(|label| !label.contains('\u{25cf}')),
        "no player row's label carries one"
    );
}

// A player row's dot the same way: a player's unsaved changes paint it
// at his row's right end in the warning colour, right of the text (the
// draft's `.plist .dot`), with no Live AATF mark to sit before.
#[test]
fn the_player_rows_dot_paints_at_the_rows_right_end() {
    let path = fixture(PesVersion::Pes21, "player_row_dot");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut entry = player_entry(&view, 70101);
    entry.basic.height = 177;
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    let mut window = Headless::new();
    let padding = f64::from(
        window
            .ctx
            .style_of(window.ctx.theme())
            .spacing
            .button_padding
            .x,
    );
    let warn = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .warn_fg_color;

    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = row(&nodes, "01 CB PLACEHOLDER ●")
        .bounds()
        .expect("the row's bounds");
    let text = text_in_rect(&shapes, "PLACEHOLDER", bounds);
    let dot = dot_in_row(&shapes, bounds);
    assert_eq!(galley_color(dot), warn, "the dot in the warning colour");
    let dot_right = f64::from(dot.pos.x + dot.galley.rect.width());
    assert!(
        (dot_right - (bounds.x1 - padding)).abs() <= 0.5,
        "the dot's right edge at the row's right less the padding ({dot_right} vs {})",
        bounds.x1 - padding
    );
    assert!(
        f64::from(dot.pos.x) >= f64::from(text.pos.x + text.galley.rect.width()),
        "the dot sits right of the text"
    );
    let row_centre = (bounds.y0 + bounds.y1) / 2.0;
    for (what, shape) in [("text", text), ("dot", dot)] {
        let centre = f64::from(shape.pos.y + shape.galley.size().y / 2.0);
        assert!(
            (centre - row_centre).abs() <= 0.5,
            "the {what} centres on the row ({centre} vs {row_centre})"
        );
    }
}

// The list is 255 points wide and not resizable, since under the design
// width the window scales rather than narrows: a drag on the list's right
// edge leaves it.
#[test]
fn a_drag_on_the_lists_right_edge_does_not_resize_it() {
    let path = fixture(PesVersion::Pes21, "list_fixed_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    // The list's column, from its frame's left edge to the end of the
    // scroll bar's strip (the list scrolls under ALL).
    let column = |window: &mut Headless, view: &mut SaveView| {
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        let width = list_scroll_bar(&nodes).x1 - f64::from(list_frame(window, &shapes).min.x);
        (width, pane_frame(&shapes, window.screen).0)
    };
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (width, before) = column(&mut window, &mut view);
    assert!(
        (width - 255.0).abs() <= 0.5,
        "the list's column is 255 wide ({width})"
    );

    // A drag from the list's right edge, the pane's left, 80 points to the
    // right.
    let from = egui::pos2(before as f32, 400.0);
    let onto = egui::pos2(before as f32 + 80.0, 400.0);
    drag(&mut window, from, onto, &mut view);
    let (width, after) = column(&mut window, &mut view);
    assert!(
        (width - 255.0).abs() <= 0.5 && (after - before).abs() <= 0.5,
        "the drag left the list at 255 ({width}) and the pane's edge where it was ({after} vs \
         {before})"
    );
}
