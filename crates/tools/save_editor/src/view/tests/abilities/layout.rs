use super::*;

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

/// The settled frame of player 701's slot 0 at the default pane: the
/// columns' widths settle over the first frames and the Cards used
/// badge's run lands.
fn settled_player_view(
    test: &str,
) -> (
    Headless,
    Vec<accesskit::Node>,
    Vec<egui::epaint::ClippedShape>,
) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    badge_landed(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    (window, nodes, shapes)
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
// points of padding and the grid's 4-point gap), the first row's squares
// 38 under the panel's title, as the draft draws them.
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
    near_draft(rows[0] - title_top, 38.0, "the first row under the title");
    for pair in rows.windows(2) {
        near_draft(pair[1] - pair[0], 43.0, "the rows' pitch");
    }
    for rating in rating_buttons(&nodes, "CB") {
        let bounds = rating.bounds().expect("bounds");
        near_draft(bounds.width(), 17.0, "a rating's width");
        near_draft(bounds.height(), 17.0, "a rating's height");
    }
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
    window.frame(vec![egui::Event::Copy], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        !window
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::CopyText(_))),
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
