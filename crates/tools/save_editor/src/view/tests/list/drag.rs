use super::*;

// SE-GUI-08
#[test]
fn a_row_dragged_above_another_trades_their_records_between_the_ids() {
    let path = fixture(PesVersion::Pes21, "drag_reorder");
    // Distinguish the two records the drag trades (the fixture's placeholders
    // are identical): a name, a stat, a boot id and a base copy each.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    file.player_mut(70101).unwrap().stats.speed = 70;
    file.player_mut(70101).unwrap().appearance.boots_id = 11;
    file.player_mut(70101).unwrap().appearance.base_copy_id = 9001;
    file.player_mut(70102).unwrap().name = "SECOND".to_owned();
    file.player_mut(70102).unwrap().stats.speed = 71;
    file.player_mut(70102).unwrap().appearance.boots_id = 12;
    file.player_mut(70102).unwrap().appearance.base_copy_id = 9002;
    file.save(&path).unwrap();
    let roster_before = file.team(701).unwrap().roster.clone();
    let first = file.player(70101).unwrap().clone();
    let second = file.player(70102).unwrap().clone();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // Select another row first, so the selection following the dragged
    // record is not vacuous.
    click(&mut window, row(&nodes, "03 CB PLACEHOLDER"), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 2)),
        "premise: row 03 selected"
    );

    // Drag player 02's row above player 01's.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let from = centre(row(&nodes, "02 CB SECOND"));
    let onto = centre(row(&nodes, "01 CB FIRST"));
    drag(&mut window, from, onto, &mut view);

    // The list shows the new order, the moved rows marked unsaved; the
    // selection followed the dragged record to its new slot.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels = row_labels(&nodes);
    assert_eq!(&labels[..2], ["01 CB SECOND ●", "02 CB FIRST ●"]);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "the selection sits on the dropped record's slot"
    );

    // Save, reopen: slot 01 still holds player 70101, whose record is the
    // one player 02 had, and the other way round; the roster is untouched.
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    let team = reopened.team(701).expect("team 701");
    assert_eq!(
        team.roster, roster_before,
        "the roster's ids and shirt numbers are as they were"
    );
    let held = reopened.player(70101).expect("player 70101");
    assert_eq!(record(held), record(&second), "slot 01's record is 70102's");
    assert_eq!(held.id, 70101);
    assert_eq!(held.appearance_id, Some(70101));
    assert_eq!(held.appearance.base_copy_id, 9001);
    let held = reopened.player(70102).expect("player 70102");
    assert_eq!(record(held), record(&first), "slot 02's record is 70101's");
    assert_eq!(held.id, 70102);
    assert_eq!(held.appearance_id, Some(70102));
    assert_eq!(held.appearance.base_copy_id, 9002);

    // The roster-order rule is unbroken.
    let view_team = aatf::TeamView::new(&reopened, 701).expect("the team views");
    let violations = aatf::Rules::official()
        .check_team(&view_team)
        .expect("the check runs")
        .violations;
    assert!(
        violations
            .iter()
            .all(|violation| !matches!(violation.kind, aatf::ViolationKind::RosterOrder { .. })),
        "no RosterOrder: {violations:?}"
    );
}

// SE-GUI-08: a filter box holding only spaces is no filter — the drag
// stays on, as it does with the box empty.
#[test]
fn a_whitespace_only_filter_keeps_the_drag_on() {
    let path = fixture(PesVersion::Pes21, "drag_whitespace_filter");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    file.player_mut(70102).unwrap().name = "SECOND".to_owned();
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.filter = " ".to_owned();
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels = row_labels(&nodes);
    assert_eq!(
        &labels[..2],
        ["01 CB FIRST", "02 CB SECOND"],
        "premise: a spaces-only filter hides no row"
    );

    let from = centre(row(&nodes, "02 CB SECOND"));
    let onto = centre(row(&nodes, "01 CB FIRST"));
    drag(&mut window, from, onto, &mut view);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels = row_labels(&nodes);
    assert_eq!(
        &labels[..2],
        ["01 CB SECOND ●", "02 CB FIRST ●"],
        "the drag traded the records under the spaces-only filter"
    );
}

// SE-GUI-08, under All teams, where each team's rows run unbroken under its
// team row: a drop on a row of the dragged player's own team moves him as
// in one team's list; a drop on another team's row or on a team row does
// nothing.
#[test]
fn under_all_a_drag_moves_within_the_team_and_nowhere_else() {
    let path = fixture(PesVersion::Pes21, "drag_reorder_all");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    file.player_mut(70102).unwrap().name = "SECOND".to_owned();
    // Team 702's first rostered player renamed too: a row's label carries
    // no team name, so the drop target's cannot be another team's too.
    let other_id = file
        .team(702)
        .unwrap()
        .roster
        .iter()
        .find(|slot| slot.player_id != 0)
        .expect("team 702 rosters someone")
        .player_id;
    file.player_mut(other_id).unwrap().name = "OTHER".to_owned();
    file.save(&path).unwrap();
    let other_label = {
        let place = file
            .team(702)
            .unwrap()
            .roster
            .iter()
            .position(|slot| slot.player_id == other_id)
            .expect("the renamed slot")
            + 1;
        let player = file.player(other_id).unwrap();
        format!(
            "{place:02} {} OTHER",
            pes_savefile::interchange::team_toml::labels::POSITIONS
                [usize::from(player.positions.registered)]
        )
    };

    let mut view = SaveView::new();
    view.open(&path);
    assert_eq!(view.team, TeamChoice::All, "open selects All teams");
    // Tall enough for team 702's block to be drawn under team 701's.
    let mut window = Headless::sized(DEFAULT_PANE.x, 1100.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // A drag onto another team's row: nothing moves.
    let before = row_labels(&nodes);
    let from = centre(row(&nodes, "02 CB SECOND"));
    let other = centre(row(&nodes, &other_label));
    drag(&mut window, from, other, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        before,
        "a drop on another team's row moved nothing"
    );
    assert!(!view.session.as_ref().unwrap().is_dirty());
    assert!(!view.session.as_ref().unwrap().can_undo());

    // A drag of the team row itself onto a player row: nothing moves.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let from = centre(team_row(&nodes, &view, 701));
    let onto = centre(row(&nodes, "01 CB FIRST"));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        before,
        "a drag of the team row moved nothing"
    );
    assert!(!view.session.as_ref().unwrap().is_dirty());
    assert!(!view.session.as_ref().unwrap().can_undo());

    // A drag onto the team row: nothing moves.
    let from = centre(row(&nodes, "02 CB SECOND"));
    let onto = centre(team_row(&nodes, &view, 701));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        before,
        "a drop on the team row moved nothing"
    );
    assert!(!view.session.as_ref().unwrap().is_dirty());
    assert!(!view.session.as_ref().unwrap().can_undo());

    // A drag onto the player's own team's row: the records trade, as in
    // one team's list.
    let from = centre(row(&nodes, "02 CB SECOND"));
    let onto = centre(row(&nodes, "01 CB FIRST"));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        &row_labels(&nodes)[..2],
        ["01 CB SECOND ●", "02 CB FIRST ●"],
        "the drop on his own team moved him"
    );
}

// Drag is off while a filter is typed, where neighbouring rows are not
// one team's neighbouring slots: under a filter listing two non-adjacent
// players of one team, a drag of one onto the other moves nothing, since
// a drop there would move the hidden records between them too.
#[test]
fn under_a_filter_a_drag_moves_nothing() {
    let path = fixture(PesVersion::Pes21, "drag_filtered");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    // Two non-adjacent players of team 701 the filter matches (slot 1
    // stays hidden).
    file.player_mut(70101).unwrap().name = "QQ FIRST".to_owned();
    file.player_mut(70103).unwrap().name = "QQ THIRD".to_owned();
    file.save(&path).unwrap();
    let roster_before = file.team(701).unwrap().roster.clone();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.filter = "qq".to_owned();
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels_before = row_labels(&nodes);
    assert_eq!(
        labels_before,
        ["01 CB QQ FIRST", "03 CB QQ THIRD"],
        "premise: the filter lists the two"
    );

    let from = centre(row(&nodes, "01 CB QQ FIRST"));
    let onto = centre(row(&nodes, "03 CB QQ THIRD"));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        labels_before,
        "the filtered rows did not move"
    );
    let session = view.session.as_ref().unwrap();
    assert_eq!(
        session.file().team(701).unwrap().roster,
        roster_before,
        "the roster is unchanged"
    );
    assert!(!session.is_dirty(), "nothing is marked unsaved");
    assert!(!session.can_undo(), "no undo entry");
}

/// A node's bounds as an egui `Rect`.
fn bounds(node: &accesskit::Node) -> egui::Rect {
    let bounds = node.bounds().expect("a row has bounds");
    egui::Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    )
}

/// A mark segment as `drop_marks` reports it: its index in the frame's
/// shapes, its two points and its stroke width.
type MarkSegment = (usize, [egui::Pos2; 2], f32);

/// The drop marks `shapes` draw in `selection` on `frame` (the list's):
/// the line's and serifs' segments (`MarkSegment`s), and any unfilled
/// rect stroked in the colour (a row's outline; a selected or hovered
/// row's frame is filled, so it cannot pass for one). A mark straddling
/// the frame's edge counts: the asserts decide what is drawn.
fn drop_marks(
    shapes: &[egui::epaint::ClippedShape],
    selection: egui::Color32,
    frame: egui::Rect,
) -> (Vec<MarkSegment>, Vec<egui::Rect>) {
    let mut segments = Vec::new();
    let mut outlines = Vec::new();
    for (index, clipped) in shapes.iter().enumerate() {
        match &clipped.shape {
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == selection
                    && frame.intersects(egui::Rect::from_points(points)) =>
            {
                segments.push((index, *points, stroke.width));
            }
            egui::Shape::Rect(shape)
                if shape.fill == egui::Color32::TRANSPARENT
                    && shape.stroke.color == selection
                    && frame.intersects(shape.rect) =>
            {
                outlines.push(shape.rect);
            }
            _ => {}
        }
    }
    (segments, outlines)
}

/// Asserts `shapes`' drop marks are the one line across `target`'s
/// edge at `edge_y`, 2 thick in `visuals`' selection colour, a serif 2
/// wide and 7 high centred on it at each of the row's edges, inside the
/// frame's border, and no outlined row.
fn assert_line(
    shapes: &[egui::epaint::ClippedShape],
    visuals: &egui::Visuals,
    frame: egui::Rect,
    target: egui::Rect,
    edge_y: f32,
    when: &str,
) {
    let (segments, outlines) = drop_marks(shapes, visuals.selection.stroke.color, frame);
    // The frame's border is stroked inside it, and a row spans the
    // frame's width: nothing of a mark reaches into the border, on
    // either side (a horizontal segment's stroke is its height; a
    // vertical one's its width).
    let border = visuals.widgets.noninteractive.bg_stroke.width;
    for (_, points, width) in &segments {
        let (lo, hi) = if points[0].x == points[1].x {
            (points[0].x - width / 2.0, points[0].x + width / 2.0)
        } else {
            (points[0].x.min(points[1].x), points[0].x.max(points[1].x))
        };
        assert!(
            lo >= frame.min.x + border - 0.01 && hi <= frame.max.x - border + 0.01,
            "no mark reaches into the frame's border {when} ({lo}..{hi} in {frame:?}, border {border})"
        );
    }
    let horizontals: Vec<([egui::Pos2; 2], f32)> = segments
        .iter()
        .filter(|(_, points, _)| points[0].y == points[1].y)
        .map(|(_, points, width)| (*points, *width))
        .collect();
    let [(line, width)] = horizontals[..] else {
        panic!("one line across the row's edge {when}, got {segments:?}")
    };
    assert_eq!(width, 2.0, "the line is 2 thick {when}");
    let y = (line[0].y + line[1].y) / 2.0;
    assert!(
        (y - edge_y).abs() < 0.01,
        "the line centred on the edge {when} ({y} vs {edge_y})"
    );
    let (x0, x1) = (line[0].x.min(line[1].x), line[0].x.max(line[1].x));
    assert!(
        (x0 - (target.min.x + border)).abs() < 0.01 && (x1 - (target.max.x - border)).abs() < 0.01,
        "the line spans the row inside the frame's border {when} ({x0}..{x1} vs {target:?}, border {border})"
    );
    let mut serif_xs: Vec<f32> = segments
        .iter()
        .filter(|(_, points, _)| points[0].x == points[1].x && points[0].y != points[1].y)
        .map(|(_, points, width)| {
            let height = (points[0].y - points[1].y).abs();
            let mid = (points[0].y + points[1].y) / 2.0;
            assert!(
                *width == 2.0 && (height - 7.0).abs() < 0.01 && (mid - y).abs() < 0.01,
                "a serif 2 wide and 7 high centred on the line {when}, got {points:?} {width} thick"
            );
            points[0].x
        })
        .collect();
    serif_xs.sort_by(|a, b| a.partial_cmp(b).expect("ordered"));
    let [left, right] = serif_xs[..] else {
        panic!("a serif at each end {when}, got {segments:?}")
    };
    assert!(
        (left - (target.min.x + border + 1.0)).abs() < 0.01
            && (right - (target.max.x - border - 1.0)).abs() < 0.01,
        "the serifs' outer sides at the border's inner side {when} ({left}, {right} vs {target:?}, border {border})"
    );
    assert!(
        outlines.is_empty(),
        "no row outlined {when}, got {outlines:?}"
    );
}

/// Asserts `shapes` draw no drop mark `when`.
fn assert_no_marks(
    shapes: &[egui::epaint::ClippedShape],
    visuals: &egui::Visuals,
    frame: egui::Rect,
    when: &str,
) {
    let (segments, outlines) = drop_marks(shapes, visuals.selection.stroke.color, frame);
    assert!(
        segments.is_empty() && outlines.is_empty(),
        "no line and no outline {when}, got {segments:?} and {outlines:?}"
    );
}

/// The mark tests' prologue: `test`'s fixture save with SECOND (slot 1)
/// and SIXTH (slot 5) renamed plus `prep`'s own changes, opened on team
/// 701 in a headless window `height` tall. Answers the save as written,
/// the view, the window and its visuals for the mark asserts.
fn mark_scene(
    test: &str,
    height: f32,
    prep: impl FnOnce(&mut EditFile),
) -> (EditFile, SaveView, Headless, egui::Visuals) {
    let path = fixture(PesVersion::Pes21, test);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70102).unwrap().name = "SECOND".to_owned();
    file.player_mut(70106).unwrap().name = "SIXTH".to_owned();
    prep(&mut file);
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let window = Headless::sized(DEFAULT_PANE.x, height);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    (file, view, window, visuals)
}

/// A press on `from` with the pointer then held to `over`: the held
/// frame's painted shapes.
fn hold_from_to(
    window: &mut Headless,
    view: &mut SaveView,
    from: egui::Pos2,
    over: egui::Pos2,
) -> Vec<egui::epaint::ClippedShape> {
    window.frame(pointer_at(from, true), |ui| show(ui, view, &|| Vec::new()));
    window
        .frame(vec![egui::Event::PointerMoved(over)], |ui| {
            show(ui, view, &|| Vec::new())
        })
        .2
}

// The place a drop on the held row moves the dragged player to is marked
// by a line between two rows, 2 thick in the selection colour with a
// serif at each end (the draft's `.dropabove` and `.dropbelow`): on the
// row's lower edge when he comes from above, on its upper edge when he
// comes from below. His own row (the drop moves nothing) and a row of
// another team under All teams get none; released off the list, none is left.
// SE-GUI-75
#[test]
fn a_held_drag_marks_where_the_drop_would_land() {
    let mut other_id = 0;
    // Tall enough for team 702's block to be drawn under team 701's at All teams.
    let (file, mut view, mut window, visuals) = mark_scene("drag_line", 1100.0, |file| {
        file.player_mut(70101).unwrap().name = "FIRST".to_owned();
        // Team 702's third rostered player renamed too: a row's label
        // carries no team name, so the next team's row cannot pass for
        // one of 701's. The third, so the drop's slot is not the dragged
        // row's slot 1 — a coincidence that would make the drop a no-op
        // whatever the team check.
        other_id = file
            .team(702)
            .unwrap()
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .nth(2)
            .expect("team 702 rosters three players")
            .player_id;
        file.player_mut(other_id).unwrap().name = "OTHER".to_owned();
    });
    let other_slot = file
        .team(702)
        .unwrap()
        .roster
        .iter()
        .position(|slot| slot.player_id == other_id)
        .expect("the renamed slot");
    assert_ne!(other_slot, 1, "premise: not the dragged row's slot 1");
    let other_label = {
        let player = file.player(other_id).unwrap();
        format!(
            "{:02} {} OTHER",
            other_slot + 1,
            pes_savefile::interchange::team_toml::labels::POSITIONS
                [usize::from(player.positions.registered)]
        )
    };

    // Player 02's drag held over player 06's row: a line across its lower
    // edge (he comes from above: the rows down to it move up and he lands
    // under them), a serif at each end, and no row outlined.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let sixth = bounds(row(&nodes, "06 CB SIXTH"));
    let first = bounds(row(&nodes, "01 CB FIRST"));
    let from = centre(row(&nodes, "02 CB SECOND"));
    let shapes = hold_from_to(&mut window, &mut view, from, sixth.center());
    assert_line(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        sixth,
        sixth.max.y,
        "over row 06",
    );

    // Held over player 01's row instead: the line across its upper edge
    // (he comes from below).
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(first.center())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_line(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        first,
        first.min.y,
        "over row 01",
    );

    // Held over his own row: the drop moves the record onto itself, so
    // no line and no outline.
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(from)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_no_marks(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        "over the dragged row itself",
    );

    // Held off the list, in the detail pane: no mark.
    let off = egui::pos2(600.0, 400.0);
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(off)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_no_marks(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        "held off the list",
    );

    // Released there: no mark left and nothing moved.
    window.frame(pointer_at(off, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_no_marks(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        "after the release off the list",
    );
    let session = view.session.as_ref().unwrap();
    assert!(
        !session.is_dirty(),
        "the release off the list moved nothing"
    );
    assert!(!session.can_undo(), "no undo entry");

    // Under ALL, held over a row of his own team first: the line shows
    // as in one team's list, so the empty next-team case cannot pass for
    // a drag never held.
    view.team = TeamChoice::All;
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let from = centre(row(&nodes, "02 CB SECOND"));
    let sixth = bounds(row(&nodes, "06 CB SIXTH"));
    let other = centre(row(&nodes, &other_label));
    let shapes = hold_from_to(&mut window, &mut view, from, sixth.center());
    assert_line(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        sixth,
        sixth.max.y,
        "over a row of his own team under All teams",
    );

    // And over his team's last player row, its lower edge where the next
    // team's row begins: that row's fill, painted after, covers none of
    // the line.
    let last_slot = file
        .team(701)
        .unwrap()
        .roster
        .iter()
        .rposition(|slot| slot.player_id != 0)
        .expect("team 701 rosters someone");
    let last = bounds(row(&nodes, &format!("{:02} CB PLACEHOLDER", last_slot + 1)));
    let next_team = bounds(team_row(&nodes, &view, 702));
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(last.center())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let frame = list_frame(&window, &shapes);
    assert_line(
        &shapes,
        &visuals,
        frame,
        last,
        last.max.y,
        "over his team's last row under All teams",
    );
    let fills: Vec<usize> = shapes
        .iter()
        .enumerate()
        .filter_map(|(index, clipped)| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == visuals.faint_bg_color
                    && shape.rect.contains(next_team.center()) =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect();
    let [fill] = fills[..] else {
        panic!("the next team's row fill alone, got {fills:?}")
    };
    let (segments, _) = drop_marks(&shapes, visuals.selection.stroke.color, frame);
    assert!(
        segments.iter().all(|(index, _, _)| *index > fill),
        "every mark shape paints after the next team row's fill (fill at {fill}, marks at {segments:?})"
    );

    // Held over a row of the next team: a drop there moves nothing, so
    // the row gets no line.
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(other)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_no_marks(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        "over another team's row under All teams",
    );
    window.frame(pointer_at(other, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let session = view.session.as_ref().unwrap();
    assert!(
        !session.is_dirty(),
        "the drop on another team moved nothing"
    );
    assert!(!session.can_undo(), "no undo entry");
}

// The mark paints once, over the rows: held over row 06 with row 07
// selected, the fill the selection paints — in row 07's own pass, after
// row 06's — covers none of the line's lower half: every mark shape
// comes after the fill's in the frame's shape list.
// SE-GUI-75
#[test]
fn the_drop_mark_paints_over_the_next_rows_fill() {
    let (_file, mut view, mut window, visuals) =
        mark_scene("drag_mark_order", DEFAULT_PANE.y, |_| {});
    // Row 07 selected before the drag starts, so it fills during its own
    // pass, after row 06's (a click would end the drag it would precede).
    view.selected = Some(Selection::Player(701, 6));

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let sixth = bounds(row(&nodes, "06 CB SIXTH"));
    let seventh = bounds(row(&nodes, "07 CB PLACEHOLDER"));
    let from = centre(row(&nodes, "02 CB SECOND"));
    let shapes = hold_from_to(&mut window, &mut view, from, sixth.center());
    // Row 07's selection fill: a rect filled in `selection.bg_fill`
    // containing the row's centre.
    let fills: Vec<usize> = shapes
        .iter()
        .enumerate()
        .filter_map(|(index, clipped)| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == visuals.selection.bg_fill
                    && shape.rect.contains(seventh.center()) =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect();
    let [fill] = fills[..] else {
        panic!("row 07's selection fill alone, got {fills:?}")
    };
    let (segments, _) = drop_marks(
        &shapes,
        visuals.selection.stroke.color,
        list_frame(&window, &shapes),
    );
    let marks: Vec<usize> = segments.iter().map(|(index, _, _)| *index).collect();
    let [line, serif_l, serif_r] = marks[..] else {
        panic!("the line and its two serifs, got {marks:?}")
    };
    assert!(
        line > fill && serif_l > fill && serif_r > fill,
        "every mark shape paints after row 07's fill (fill at {fill}, marks at {marks:?})"
    );
}

// Held with the pointer on the very edge rows 05 and 06 share: both
// rows' bounds take the edge, but the mark is the upper row's alone —
// row 05's lower edge — the row a release there drops on.
// SE-GUI-75
#[test]
fn on_a_shared_edge_the_upper_rows_mark_shows_and_drops() {
    let (_file, mut view, mut window, visuals) =
        mark_scene("drag_shared_edge", DEFAULT_PANE.y, |_| {});

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let fifth = bounds(row(&nodes, "05 CB PLACEHOLDER"));
    let from = centre(row(&nodes, "02 CB SECOND"));
    // The pointer on the edge the rows share: row 05's lower bound is
    // row 06's upper.
    let edge = egui::pos2(fifth.center().x, fifth.max.y);
    let shapes = hold_from_to(&mut window, &mut view, from, edge);
    assert_line(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        fifth,
        fifth.max.y,
        "on the edge rows 05 and 06 share",
    );

    // Released there, the drop lands on the upper row's slot: SECOND
    // takes place 05.
    window.frame(pointer_at(edge, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        &row_labels(&nodes)[4..6],
        ["05 CB SECOND ●", "06 CB SIXTH"],
        "the drop on the shared edge lands on the upper row"
    );
}

// Held on the edge the dragged row shares with the row below: both
// bounds take the pointer, the mark is the next row's alone (its lower
// edge, the player coming from above), and the release there must reach
// it, not stop at the dragged row, whose own drop moves nothing.
// SE-GUI-75, SE-GUI-08
#[test]
fn on_the_dragged_rows_lower_edge_the_release_reaches_the_marked_row() {
    let (_file, mut view, mut window, visuals) =
        mark_scene("drag_edge_release", DEFAULT_PANE.y, |_| {});

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let fifth = bounds(row(&nodes, "05 CB PLACEHOLDER"));
    let sixth = bounds(row(&nodes, "06 CB SIXTH"));
    // Row 05 dragged to the edge it shares with row 06 below it: its
    // lower bound is row 06's upper.
    let edge = egui::pos2(fifth.center().x, fifth.max.y);
    let shapes = hold_from_to(&mut window, &mut view, fifth.center(), edge);
    assert_line(
        &shapes,
        &visuals,
        list_frame(&window, &shapes),
        sixth,
        sixth.max.y,
        "on the dragged row's lower edge",
    );

    // Released there, the drop reaches the marked row: the dragged
    // record lands on place 06 where the line stood, and the record that
    // stood there moves up to place 05.
    window.frame(pointer_at(edge, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        &row_labels(&nodes)[4..6],
        ["05 CB SIXTH ●", "06 CB PLACEHOLDER ●"],
        "the release reached the marked row: the drop landed on place 06"
    );
}

// The dragged row is drawn under the pointer, the point first pressed
// kept under it, as a dragged lineup row is: row 03 pressed 20 points
// right of its name's start and moved by a known offset paints its
// handle, place, position and name once each, at their rest positions
// plus that offset, the name after row 04's place (a layer over the list,
// not the list's own), and nothing inside its own place. The frame after
// the release paints the row at rest again, in the list. With a filter
// typed the row does not drag: the same press and move paint the row at
// rest alone.
// SE-GUI-69
#[test]
fn a_dragged_row_floats_with_the_point_pressed_under_the_pointer() {
    /// The texts painted inside `bounds` (a row's), in paint order, each
    /// with its position.
    fn texts_inside(
        shapes: &[egui::epaint::ClippedShape],
        bounds: egui::Rect,
    ) -> Vec<(String, egui::Pos2)> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if bounds.expand(0.5).contains(shape.pos) => {
                    Some((shape.galley.job.text.clone(), shape.pos))
                }
                _ => None,
            })
            .collect()
    }
    /// The paint-order index and text of each text painted within half a
    /// point of `at`.
    fn texts_at(shapes: &[egui::epaint::ClippedShape], at: egui::Pos2) -> Vec<(usize, &str)> {
        shapes
            .iter()
            .enumerate()
            .filter_map(|(index, clipped)| match &clipped.shape {
                egui::Shape::Text(shape) if shape.pos.distance(at) <= 0.5 => {
                    Some((index, shape.galley.job.text.as_str()))
                }
                _ => None,
            })
            .collect()
    }

    let path = fixture(PesVersion::Pes21, "drag_float");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70103).unwrap().name = "DRAGGED NAME".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let offset = egui::vec2(30.0, 40.0);
    for (filter, floats) in [("", true), ("dragged", false)] {
        view.filter = filter.to_owned();
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let bounds = row(&nodes, "03 CB DRAGGED NAME")
            .bounds()
            .expect("row 03's bounds");
        let bounds = egui::Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        );
        let rest = texts_inside(&shapes, bounds);
        let texts: Vec<&str> = rest.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(
            texts,
            ["\u{2261}", "03", "CB", "DRAGGED NAME"],
            "premise: row 03 paints its handle, place, position and name (filter {filter:?})"
        );
        // Each of the row's four texts painted once, `shift` from its rest
        // position; their paint-order indices returned, in `rest`'s order.
        let assert_row_at = |shapes: &[egui::epaint::ClippedShape],
                             shift: egui::Vec2,
                             when: &str| {
            rest.iter()
                .map(|(text, at)| {
                    let here: Vec<usize> = texts_at(shapes, *at + shift)
                        .into_iter()
                        .filter(|(_, painted)| painted == text)
                        .map(|(index, _)| index)
                        .collect();
                    let [index] = here[..] else {
                        panic!("{text:?} painted once at rest plus {shift:?} {when} (filter {filter:?}), got {here:?}")
                    };
                    index
                })
                .collect::<Vec<usize>>()
        };
        // Nothing painted where the row's texts would float to.
        let assert_nothing_moved = |shapes: &[egui::epaint::ClippedShape], when: &str| {
            for (text, at) in &rest {
                let there = texts_at(shapes, *at + offset);
                assert!(
                    there.is_empty(),
                    "nothing at {text:?}'s rest plus the move {when} (filter {filter:?}), got {there:?}"
                );
            }
        };
        // Row 04's place's paint-order index (unfiltered, where row 04
        // is listed): the list's own layer paints it, so a text painted
        // after it floats above the list.
        let place_04_index = |shapes: &[egui::epaint::ClippedShape]| {
            let place_04 = nodes
                .iter()
                .find(|node| {
                    player_row(node) && node.label().is_some_and(|label| label.starts_with("04 "))
                })
                .and_then(accesskit::Node::bounds)
                .expect("row 04's bounds");
            shapes
                .iter()
                .position(|clipped| match &clipped.shape {
                    egui::Shape::Text(shape) => {
                        shape.galley.job.text == "04"
                            && f64::from(shape.pos.y) >= place_04.y0
                            && f64::from(shape.pos.y) <= place_04.y1
                    }
                    _ => false,
                })
                .expect("row 04's place painted")
        };

        let name_rest = rest[3].1;
        let press = egui::pos2(
            name_rest.x + 20.0,
            centre(row(&nodes, "03 CB DRAGGED NAME")).y,
        );
        window.frame(pointer_at(press, true), |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
        let (_, _, held) = window.frame(vec![egui::Event::PointerMoved(press + offset)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
        if floats {
            let inside = texts_inside(&held, bounds);
            assert!(
                inside.is_empty(),
                "row 03's place left blank while dragged, got {inside:?}"
            );
            let name = assert_row_at(&held, offset, "while dragged")[3];
            let place = place_04_index(&held);
            assert!(
                name > place,
                "the dragged name paints over the list, after row 04's place ({name} vs {place})"
            );
        } else {
            assert_row_at(&held, egui::Vec2::ZERO, "under a filter");
            assert_nothing_moved(&held, "under a filter");
        }

        // Released where it was pressed: the drop moves nothing, and the
        // frame after the release paints the row at rest, in the list.
        window.frame(pointer_at(press, false), |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
        let (_, _, after) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let name = assert_row_at(&after, egui::Vec2::ZERO, "after the release")[3];
        assert_nothing_moved(&after, "after the release");
        if floats {
            let place = place_04_index(&after);
            assert!(
                name < place,
                "after the release the name paints in the list, before row 04's place ({name} vs {place})"
            );
        }
        assert!(
            !view.session.as_ref().unwrap().is_dirty(),
            "the drag moved no record"
        );
    }
}

// A draggable row shows the pointing hand wherever the pointer is on it
// (egui leaves a row that also takes clicks on the arrow, so the row asks
// for it); under a filter the row does not drag, and the pointer is the
// arrow.
// SE-GUI-64
#[test]
fn a_draggable_rows_pointer_is_the_pointing_hand() {
    let path = fixture(PesVersion::Pes21, "drag_cursor");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let cursor_at = |window: &mut Headless, view: &mut SaveView, at: egui::Pos2| {
        window
            .frame(vec![egui::Event::PointerMoved(at)], |ui| {
                show(ui, view, &|| Vec::new());
                ui.ctx().output(|output| output.cursor_icon)
            })
            .0
    };
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(row(&nodes, "01 CB FIRST"));
    cursor_at(&mut window, &mut view, at);
    assert_eq!(
        cursor_at(&mut window, &mut view, at),
        egui::CursorIcon::PointingHand,
        "a draggable row's pointer is the pointing hand"
    );

    // A filter typed: the row does not drag, so the pointer is the arrow.
    view.filter = "first".to_owned();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(row(&nodes, "01 CB FIRST"));
    cursor_at(&mut window, &mut view, at);
    assert_eq!(
        cursor_at(&mut window, &mut view, at),
        egui::CursorIcon::Default,
        "a filtered row's pointer is the arrow"
    );
}
