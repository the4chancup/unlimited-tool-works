use super::*;

// Every panel title of the view stands `TITLE_GAP` above the first thing
// drawn under it — a field's frame, a checkbox's painted square, the
// positions' label row's text — as the team view's titles do: the card
// panels' `.skcols` margin and the positions' `#pos-layout` margin add
// their own over it, per the draft.
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
    // The positions panel's first drawn text is its selector's, which the
    // draft's `#pos-layout` `margin-top` stands lower than a first field
    // row; the card panels' squares take the `.skcols` `margin-top`'s
    // extra half point.
    for (title, under, gap) in [
        (
            "General info",
            first_rect_under(
                &nodes,
                &shapes,
                "General info",
                right("General info"),
                false,
            ),
            6.0,
        ),
        (
            "Other stats",
            first_rect_under(&nodes, &shapes, "Other stats", right("Other stats"), false),
            6.0,
        ),
        (
            "Playable positions",
            first_text_under(
                &nodes,
                &shapes,
                "Playable positions",
                right("Playable positions"),
            ),
            7.5,
        ),
        (
            "Skills",
            first_rect_under(&nodes, &shapes, "Skills", right("Skills"), true),
            6.5,
        ),
        (
            "Tricks",
            first_rect_under(&nodes, &shapes, "Tricks", right("Tricks"), true),
            6.5,
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
            6.5,
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
            6.0,
        ),
    ] {
        let bottom = label_bounds(&nodes, title).y1;
        // The insets the panels subtract are estimates: a title may stand
        // a hair under the gap ("Playable positions" stands 5.84), never
        // over it.
        assert!(
            (-0.25..=0.0).contains(&(under - bottom - gap)),
            "{title} stands {gap} above its first drawn row ({under} vs {bottom})"
        );
    }
}

// The sections' places under the title's line, at the draft's: the
// columns' first frames `GROUP_GAP` under the title's row (its
// `.pcols > *` `margin-top`) and every heading's text 7 inside its
// frame, so every heading's text top stands 20 under the line;
// under a heading the first field rows' labels 22.5 and Height's 70.5
// (the `.col` gap, the `.gen` margin and the rows' pitch), the first
// checkbox's box 19.5 (the `.skcols` `margin-top`); the layout toggle
// 22.5 (`#pos-layout`'s own margin); and COM playing styles' heading
// where the draft's stands under Tricks'.
#[test]
fn the_stats_screens_sections_stand_at_the_drafts_offsets() {
    let (window, nodes, shapes) = settled_player_view("stats_offsets");
    let (line, _) = line_and_pane_top(&window, &nodes, &shapes);
    let heading = |title: &str| label_bounds(&nodes, title).y0;
    for title in ["General info", "Ability stats"] {
        near_draft(
            heading(title) - line,
            20.0,
            &format!("{title}'s heading under the line"),
        );
    }
    for (what, offset, draft) in [
        (
            "Height under General info's heading",
            label_bounds(&nodes, "Height").y0 - heading("General info"),
            70.5,
        ),
        (
            "Attacking Prowess under Ability stats' heading",
            label_bounds(&nodes, "Attacking Prowess").y0 - heading("Ability stats"),
            22.5,
        ),
        (
            "the layout toggle under Playable positions' heading",
            layout_segments(&nodes)[0]
                .bounds()
                .expect("the Pitch segment's bounds")
                .y0
                - heading("Playable positions"),
            22.5,
        ),
    ] {
        assert!(
            (offset - draft).abs() <= 0.5,
            "{what}: {offset} against the draft's {draft}"
        );
    }
    // The first checkbox's box at the draft's, the `.skcols` own
    // margin's half point made out.
    let heading_check = node_labeled(&nodes, Role::CheckBox, "Heading")
        .bounds()
        .expect("the Heading checkbox's bounds")
        .y0
        - heading("Skills");
    assert!(
        (heading_check - 19.5).abs() <= 0.25,
        "the Heading checkbox under Skills' heading: {heading_check} against the draft's 19.5"
    );
    near_draft(
        heading("COM playing styles") - heading("Tricks"),
        169.5,
        "COM playing styles' heading under Tricks'",
    );
}

/// `actual` within a point of the draft's `draft`, `what` naming it.
#[track_caller]
fn near_draft(actual: f64, draft: f64, what: &str) {
    assert!(
        (actual - draft).abs() <= 1.0,
        "{what}: {actual} against the draft's {draft}"
    );
}

// The positions grid at the draft's pitch (`#pos-grid`): each rating
// square 17 by 17, the cells of a row 66 apart (the 62-point `.poscell`
// and the `.posrow`'s 4-point gap), the rows 43 apart (a cell's 2 and 3
// points of padding and the grid's 4-point gap), the rows centred in the
// grid's List-height region — the first row's squares ~91 under the
// panel's title here.
#[test]
fn the_positions_grid_stands_at_the_drafts_pitch() {
    let (_, nodes, _) = settled_player_view("positions_pitch");
    let title_top = label_bounds(&nodes, "Playable positions").y0;
    let c_of = |position: &str| {
        rating_buttons(&nodes, position)[0]
            .bounds()
            .expect("bounds")
    };
    for row in [
        &["LWF", "CF", "SS", "RWF"][..],
        &["LMF", "AMF", "RMF"],
        &["LB", "CB", "RB"],
    ] {
        for pair in row.windows(2) {
            near_draft(
                c_of(pair[1]).x0 - c_of(pair[0]).x0,
                66.0,
                &format!("{} to {}", pair[0], pair[1]),
            );
        }
    }
    let rows = ["LWF", "LMF", "CMF", "LB", "GK"].map(|position| c_of(position).y0);
    near_draft(rows[0] - title_top, 90.9, "the first row under the title");
    for pair in rows.windows(2) {
        near_draft(pair[1] - pair[0], 43.0, "the rows' pitch");
    }
    for rating in rating_buttons(&nodes, "CB") {
        let bounds = rating.bounds().expect("bounds");
        near_draft(bounds.width(), 17.0, "a rating's width");
        near_draft(bounds.height(), 17.0, "a rating's height");
    }
}

// The positions grid keeps List's height in every layout (thirteen
// 17-point rows a point apart, the draft's `#pos-grid` `min-height`), so
// a layout switch never moves the panel's bottom — and in Pitch the
// rows stand centred in the grid's height, the first row as far under
// its top as the last is above its bottom.
#[test]
fn the_positions_grid_keeps_lists_height_in_every_layout() {
    let (mut view, mut window) = positions_view("grid_height", DEFAULT_PANE.x);
    let mut heights = Vec::new();
    let mut pitch_gaps = None;
    for at in 0..3 {
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let frame = group_frame(&window, &nodes, &shapes, "Playable positions");
        heights.push(frame.height());
        if at == 0 {
            // The grid's top is `TOGGLE_GRID_GAP` under the segments,
            // its bottom the frame's inside (the frame's lower margin
            // and stroke); the rows' first top is LWF's label, the
            // last's bottom GK's squares.
            let group = egui::Frame::group(&window.ctx.global_style());
            let below = f32::from(group.inner_margin.bottom) + group.stroke.width;
            let grid_top = layout_segments(&nodes)
                .iter()
                .map(|node| node.bounds().expect("bounds").y1)
                .fold(0.0, f64::max)
                + f64::from(crate::view::abilities::TOGGLE_GRID_GAP);
            let grid_bottom = f64::from(frame.max.y - below);
            let first = label_bounds(&nodes, "LWF").y0;
            let last = rating_buttons(&nodes, "GK")[0].bounds().expect("bounds").y1;
            pitch_gaps = Some((first - grid_top, grid_bottom - last));
        }
        if at < 2 {
            click(&mut window, layout_segments(&nodes)[at + 1], &mut view);
        }
    }
    for pair in heights.windows(2) {
        assert!(
            (pair[0] - pair[1]).abs() <= 0.5,
            "the frame's height kept across layouts ({heights:?})"
        );
    }
    let (top_gap, bottom_gap) = pitch_gaps.expect("the Pitch gaps were measured");
    assert!(
        (top_gap - bottom_gap).abs() <= 0.5,
        "Pitch's rows centred in the grid's height ({top_gap} over, {bottom_gap} under)"
    );
}

/// The layout toggle's segments under the Playable positions title, in x
/// order (the foot/hand rows' own `Left` sits in General info, above the
/// title, so the filter by row).
fn layout_segments(nodes: &[accesskit::Node]) -> Vec<&accesskit::Node> {
    let title_bottom = label_bounds(nodes, "Playable positions").y1;
    let mut segments: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && matches!(node.label(), Some("Pitch" | "Left" | "List"))
                && node.bounds().is_some_and(|b| b.y0 > title_bottom)
        })
        .collect();
    segments.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    segments
}

/// The `C` `B` `A` buttons on `label`'s row to its right, in x order (the
/// List layout's ratings, which stand beside their label rather than
/// under it).
fn row_ratings(nodes: &[accesskit::Node], label: accesskit::Rect) -> Vec<&accesskit::Node> {
    let centre_y = (label.y0 + label.y1) / 2.0;
    let mut ratings: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && matches!(node.label(), Some("C" | "B" | "A"))
                && node.bounds().is_some_and(|b| {
                    ((b.y0 + b.y1) / 2.0 - centre_y).abs() < 4.0 && b.x0 > label.x1 - 1.0
                })
        })
        .collect();
    ratings.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    ratings
}

/// Whether the frame draws no scroll bar in the detail pane (a bar left
/// of x 300 is the player list's own).
fn detail_scroll_free(nodes: &[accesskit::Node]) -> bool {
    !nodes
        .iter()
        .any(|node| node.role() == Role::ScrollBar && node.bounds().is_some_and(|b| b.x0 > 300.0))
}

// The layout toggle under the Playable positions title (the draft's
// `#pos-layout`), Pitch held on a fresh editor: Left starts each of the
// five rows' first cells at one left edge, the block the widest row
// makes centred in the panel; List draws the 13 positions one a row in
// the draft's `POS_LIST` order, each label's left edge at one x, its
// C B A right of it. The detail pane shows whole in all three at the
// default window.
// SE-GUI-85
#[test]
fn the_layout_toggle_picks_one_of_three_grids() {
    let (mut view, mut window) = positions_view("layout_toggle", DEFAULT_PANE.x);
    let nodes = draw(&mut window, &mut view);
    let segments = layout_segments(&nodes);
    assert_eq!(
        segments
            .iter()
            .map(|node| node.label().unwrap_or("?"))
            .collect::<Vec<_>>(),
        ["Pitch", "Left", "List"],
        "the toggle reads Pitch Left List"
    );
    assert_eq!(
        segments[0].toggled(),
        Some(accesskit::Toggled::True),
        "Pitch held with the setting absent"
    );
    assert!(
        detail_scroll_free(&nodes),
        "Pitch: no scroll bar in the detail pane"
    );

    // Left: the five rows' first cells share one left x (each cell's
    // ratings stand 3.5 in from its edge) and the widest row's block is
    // centred in the panel.
    click(&mut window, segments[1], &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        layout_segments(&nodes)[1].toggled(),
        Some(accesskit::Toggled::True),
        "the press holds Left"
    );
    let c_of = |position: &str| {
        rating_buttons(&nodes, position)[0]
            .bounds()
            .expect("bounds")
    };
    let edge = c_of("LWF").x0;
    for position in ["LMF", "CMF", "LB", "GK"] {
        assert!(
            (c_of(position).x0 - edge).abs() < 0.5,
            "{position}'s cell starts at the block's left edge ({} vs {edge})",
            c_of(position).x0
        );
    }
    let column_centre = f64::from(
        group_frame(&window, &nodes, &shapes, "Playable positions")
            .center()
            .x,
    );
    let block_centre = (c_of("LWF").x0
        + rating_buttons(&nodes, "RWF")[2]
            .bounds()
            .expect("bounds")
            .x1)
        / 2.0;
    assert!(
        (block_centre - column_centre).abs() <= 1.0,
        "the widest row's block centred in the panel ({block_centre} vs {column_centre})"
    );
    assert!(
        detail_scroll_free(&nodes),
        "Left: no scroll bar in the detail pane"
    );

    // List: the 13 positions one a row in `POSITION_LIST`'s order, every
    // label's left edge at the column's x, its three ratings right of it
    // and on its row.
    click(&mut window, layout_segments(&nodes)[2], &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        layout_segments(&nodes)[2].toggled(),
        Some(accesskit::Toggled::True),
        "the press holds List"
    );
    const LIST_ORDER: [&str; 13] = [
        "CF", "SS", "LWF", "RWF", "AMF", "LMF", "RMF", "CMF", "DMF", "LB", "RB", "CB", "GK",
    ];
    let mut top = f64::MIN;
    let mut left = None;
    for position in LIST_ORDER {
        let label = label_bounds(&nodes, position);
        assert!(label.y0 > top + 3.0, "{position} on its own row");
        top = label.y0;
        match left {
            None => left = Some(label.x0),
            Some(left) => assert!(
                (label.x0 - left).abs() < 0.5,
                "{position}'s label at the column's left x ({left} vs {})",
                label.x0
            ),
        }
        let ratings = row_ratings(&nodes, label);
        assert_eq!(ratings.len(), 3, "{position}'s three ratings on its row");
        assert!(
            ratings[0].bounds().expect("bounds").x0 >= label.x1 - 1.0,
            "{position}'s ratings right of its label"
        );
    }
    // The first row's square stands centred in the grid's List-height
    // region — ~47.5 under the title here — and the rows at the draft's
    // pitch.
    let title_top = label_bounds(&nodes, "Playable positions").y0;
    let squares = LIST_ORDER.map(|position| {
        row_ratings(&nodes, label_bounds(&nodes, position))[0]
            .bounds()
            .expect("bounds")
            .y0
    });
    near_draft(
        squares[0] - title_top,
        47.5,
        "the first List row under the title",
    );
    for pair in squares.windows(2) {
        near_draft(pair[1] - pair[0], 19.75, "the List rows' pitch");
    }
    assert!(
        detail_scroll_free(&nodes),
        "List: no scroll bar in the detail pane"
    );
}

// A rating not chosen paints its letter in the normal text colour, which
// is darker than a resting button's own grey, so the chosen one stands
// out (the draft's `.rate:not(.off) .sel:not(.on)`).
// SE-GUI-85
#[test]
fn an_unchosen_rating_paints_the_normal_text_colour() {
    let (mut view, mut window) = positions_view("unchosen_letter", DEFAULT_PANE.x);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let visuals = &window.ctx.style_of(window.ctx.theme()).visuals;
    let text_color = visuals.text_color();
    let resting = visuals.widgets.inactive.fg_stroke.color;
    assert_ne!(
        resting, text_color,
        "premise: a resting button's grey differs from the text colour"
    );
    // 70101's LWF holds C (`a_playable_positions_rating_writes_its_own_slot_and_undoes`'s
    // premise): its B letter is one not chosen.
    let b = rating_buttons(&nodes, "LWF")[1].bounds().expect("bounds");
    assert_eq!(
        painted_text_color(&shapes, "B", b),
        text_color,
        "the unchosen B letter in the normal text colour"
    );
}

// The Skills header's row is one line of text high, as the draft's: the
// title, "Cards used for" and the chip's text on the column titles' line,
// 8 apart (the draft's `.row` gap, egui's item spacing; the chip's text 4
// more inside it), and
// the first checkbox row 19.5 under the title's top.
#[test]
fn the_skills_header_is_one_text_line_high() {
    let (_, nodes, _) = settled_player_view("skills_header");
    let skills = label_bounds(&nodes, "Skills");
    let cards_used = label_bounds(&nodes, "Cards used for");
    let chip = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Label && node.value().is_some_and(|value| value.contains(" of "))
        })
        .and_then(accesskit::Node::bounds)
        .expect("the chip's text");
    near_draft(skills.height(), 15.0, "the Skills title's height");
    near_draft(cards_used.y0, skills.y0, "Cards used for's top");
    near_draft(chip.y0, skills.y0, "the chip's text top");
    near_draft(cards_used.x0 - skills.x1, 8.0, "Skills to Cards used for");
    near_draft(
        chip.x0 - cards_used.x1,
        12.0,
        "Cards used for to the chip's text",
    );
    for card in ["Heading", "Man Marking"] {
        let row = node_labeled(&nodes, Role::CheckBox, card)
            .bounds()
            .expect("bounds");
        near_draft(
            row.y0 - skills.y0,
            19.5,
            &format!("{card}'s row under the title"),
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

// A drag across a position's label selects no text: the labels opt out
// of egui's selectable labels (a drag past the click's 6-point reach is
// no click either, so it registers nothing).
#[test]
fn a_drag_across_a_positions_label_selects_no_text() {
    let path = fixture(PesVersion::Pes21, "positions_label_drag");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let registered = |view: &SaveView| player_entry(view, 70101).positions.registered;
    assert_eq!(registered(&view), 1, "premise: registered CB");

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let ss = label_bounds(&nodes, "SS");
    let mid = ((ss.y0 + ss.y1) / 2.0) as f32;
    drag(
        &mut window,
        egui::pos2(ss.x0 as f32 + 1.0, mid),
        egui::pos2(ss.x1 as f32 - 1.0, mid),
        &mut view,
    );
    // Ctrl+C with no text selected copies the selected player, a text
    // opening `# Unlimited Tool Works:`; a selection would have copied
    // its own text.
    window.frame(vec![egui::Event::Copy], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        !window.commands.iter().any(|command| matches!(
            command,
            egui::OutputCommand::CopyText(text) if !text.starts_with("# Unlimited Tool Works: ")
        )),
        "the drag selected no text: {:?}",
        window.commands
    );
    assert_eq!(registered(&view), 1, "the drag is not a click");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "no edit was pushed"
    );
}

// The player view's two-choice toggles (the draft's `.rate.lr`): Stronger
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

// The frame that first shows a player draws him where every later frame
// does: its ability stats grid is new, so egui lays the frame's first
// pass out narrow and discards it, and the pass that reaches the screen
// must not lay out on the widths that discarded pass measured.
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

    // Each pass of the frame selecting the next player: the columns'
    // widths the pass reads, the widths it measures, and whether egui
    // discards it.
    let stored = |ui: &egui::Ui| {
        ui.ctx().data(|data| {
            data.get_temp::<[f32; 2]>(egui::Id::unique(crate::view::abilities::COLUMN_WIDTHS_ID))
        })
    };
    view.selected = Some(Selection::Player(734, 1));
    let mut passes = Vec::new();
    let (_, first, _) = window.frame(Vec::new(), |ui| {
        let read = stored(ui);
        show(ui, &mut view, &|| Vec::new());
        let measured = stored(ui);
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

// The Stats screen's stacked controls are spaced by the gaps between
// their boxes: its 18-point fields leave 6 between them, standing 24
// apart top to top (General info, between its blocks as inside them, and
// Ability stats), and its stacked checkboxes leave 8 between their
// 14-point squares, their 18-point rows standing 22 apart.
#[test]
fn the_stats_screens_fields_stand_24_apart_and_its_checkboxes_22() {
    let (_, nodes, _) = settled_player_view("stats_gaps");
    let top = |node: &accesskit::Node| node.bounds().expect("drawn bounds").y0;
    let first_right_of = |label: &str, role: Role| {
        top(right_of_label(&nodes, label, role)
            .first()
            .unwrap_or_else(|| panic!("a {role:?} right of {label}")))
    };
    let name = top(field_right_of_label(&nodes, "Name (", Role::TextInput));
    let shirt = top(field_right_of_label(&nodes, "Shirt (", Role::TextInput));
    let height = first_right_of("Height", Role::SpinButton);
    let age = first_right_of("Age", Role::SpinButton);
    let foot = first_right_of("Stronger foot", Role::Button);
    let hand = first_right_of("Stronger hand", Role::Button);
    let registered = first_right_of("Registered position", Role::ComboBox);
    let stats: Vec<String> = pes_savefile::interchange::team_toml::labels::ABILITY_STATS[..2]
        .iter()
        .map(|name| pes_savefile::interchange::team_toml::labels::title_case(name))
        .collect();
    let first_stat = first_right_of(&stats[0], Role::SpinButton);
    let second_stat = first_right_of(&stats[1], Role::SpinButton);
    // The checkbox under Heading, the first card of the Skills' first
    // column: the nearest one below it at its x.
    let heading = node_labeled(&nodes, Role::CheckBox, "Heading")
        .bounds()
        .expect("Heading's bounds");
    let under_heading = nodes
        .iter()
        .filter(|node| node.role() == Role::CheckBox)
        .filter_map(accesskit::Node::bounds)
        .filter(|b| (b.x0 - heading.x0).abs() <= 0.5 && b.y0 > heading.y0)
        .map(|b| b.y0)
        .fold(f64::INFINITY, f64::min);
    for (upper, lower, expected, what) in [
        (name, shirt, 24.0, "the Shirt box under the Name box"),
        (shirt, height, 24.0, "the Height box under the Shirt box"),
        (age, foot, 24.0, "the Stronger foot row under the Age box"),
        (
            hand,
            registered,
            24.0,
            "the Registered position combo under the Stronger hand row",
        ),
        (
            first_stat,
            second_stat,
            24.0,
            "the second ability stat's box under the first's",
        ),
        (
            heading.y0,
            under_heading,
            22.0,
            "the checkbox under Heading's",
        ),
    ] {
        assert!(
            (lower - upper - expected).abs() <= 0.5,
            "{what}: {} apart, expected {expected}",
            lower - upper
        );
    }
}
