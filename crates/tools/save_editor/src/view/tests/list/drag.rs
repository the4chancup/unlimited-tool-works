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

// SE-GUI-08, under ALL, where each team's rows run unbroken under its
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
    assert_eq!(view.team, TeamChoice::All, "open selects ALL");
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

// The draft's `.dragover`: the row a drop on it would move the dragged
// player to is outlined in the selection colour while the drag is held
// over it. His own row (the drop moves nothing) and a row of another
// team under ALL get none; released off the list, no row is.
// SE-GUI-64
#[test]
fn a_held_drag_outlines_only_the_row_it_would_land_on() {
    /// The bands `shapes` outline in `selection` inside `frame` (the
    /// list's): the unfilled rects stroked in it (a selected or hovered
    /// row's frame is filled, so it cannot pass for the outline).
    fn outlined(
        shapes: &[egui::epaint::ClippedShape],
        selection: egui::Color32,
        frame: egui::Rect,
    ) -> Vec<egui::Rect> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.fill == egui::Color32::TRANSPARENT
                        && shape.stroke.color == selection
                        && frame.contains_rect(shape.rect) =>
                {
                    Some(shape.rect)
                }
                _ => None,
            })
            .collect()
    }

    let path = fixture(PesVersion::Pes21, "drag_outline");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    file.player_mut(70103).unwrap().name = "THIRD".to_owned();
    // Team 702's second rostered player renamed too: a row's label
    // carries no team name, so the next team's row cannot pass for one of
    // 701's. The second, not the first, so the drop's slot is not the
    // dragged row's slot 0 — a coincidence that would make the drop a
    // no-op whatever the team check.
    let other_id = file
        .team(702)
        .unwrap()
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .nth(1)
        .expect("team 702 rosters two players")
        .player_id;
    file.player_mut(other_id).unwrap().name = "OTHER".to_owned();
    file.save(&path).unwrap();
    let other_slot = file
        .team(702)
        .unwrap()
        .roster
        .iter()
        .position(|slot| slot.player_id == other_id)
        .expect("the renamed slot");
    assert_ne!(other_slot, 0, "premise: not the dragged row's slot 0");
    let other_label = {
        let player = file.player(other_id).unwrap();
        format!(
            "{:02} {} OTHER",
            other_slot + 1,
            pes_savefile::interchange::team_toml::labels::POSITIONS
                [usize::from(player.positions.registered)]
        )
    };

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    // Tall enough for team 702's block to be drawn under team 701's at ALL.
    let mut window = Headless::sized(DEFAULT_PANE.x, 1100.0);
    let selection = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .selection
        .stroke
        .color;

    // Player 01's drag held over player 03's row: that row's band alone
    // is outlined.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let target = row(&nodes, "03 CB THIRD")
        .bounds()
        .expect("player 03's row");
    let row_height = (target.y1 - target.y0) as f32;
    let from = centre(row(&nodes, "01 CB FIRST"));
    let over = centre(row(&nodes, "03 CB THIRD"));
    window.frame(pointer_at(from, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(over)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let outlines = outlined(&shapes, selection, list_frame(&window, &shapes));
    let [outline] = outlines.as_slice() else {
        panic!("player 03's band alone is outlined, got {outlines:?}")
    };
    assert!(
        outline.contains(over),
        "the outline is the band under the pointer ({outline:?}, centre {over:?})"
    );
    assert!(
        outline.height() <= row_height + 1.0,
        "the outline is one row, not a run of rows ({outline:?} vs {row_height})"
    );

    // Held over his own row: the drop moves the record onto itself, so
    // no row is outlined.
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(from)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        outlined(&shapes, selection, list_frame(&window, &shapes)).is_empty(),
        "a drag over its own row outlines nothing"
    );

    // Released off the list, in the detail pane: no row is outlined and
    // nothing moves.
    let off = egui::pos2(600.0, 400.0);
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(off)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        outlined(&shapes, selection, list_frame(&window, &shapes)).is_empty(),
        "held off the list outlines no row"
    );
    window.frame(pointer_at(off, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        outlined(&shapes, selection, list_frame(&window, &shapes)).is_empty(),
        "the drag ended: no row is outlined"
    );
    let session = view.session.as_ref().unwrap();
    assert!(
        !session.is_dirty(),
        "the release off the list moved nothing"
    );
    assert!(!session.can_undo(), "no undo entry");

    // Under ALL, held over a row of the next team: a drop there moves
    // nothing, so the row gets no outline.
    view.team = TeamChoice::All;
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let from = centre(row(&nodes, "01 CB FIRST"));
    let other = centre(row(&nodes, &other_label));
    window.frame(pointer_at(from, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(other)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        outlined(&shapes, selection, list_frame(&window, &shapes)).is_empty(),
        "a drag over another team's row outlines nothing"
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

// The dragged row is drawn under the pointer, the point first pressed
// kept under it, as a dragged lineup row is: row 03 pressed 20 points
// right of its name's start and moved by a known offset paints its name
// once, at its rest position plus that offset, after row 04's place (a
// layer over the list, not the list's own). With a filter typed the row
// does not drag: the same press and move paint the name at rest alone.
// SE-GUI-69
#[test]
fn a_dragged_row_floats_with_the_point_pressed_under_the_pointer() {
    let path = fixture(PesVersion::Pes21, "drag_float");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70103).unwrap().name = "DRAGGED NAME".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    // The name's paints over the list's frame, with their place in the
    // paint order: once row 03 is selected, the detail pane draws his name
    // too, outside it.
    let names_in_list = |window: &Headless, shapes: &[egui::epaint::ClippedShape]| {
        let frame = list_frame(window, shapes);
        shapes
            .iter()
            .enumerate()
            .filter_map(|(index, clipped)| match &clipped.shape {
                egui::Shape::Text(shape)
                    if shape.galley.job.text == "DRAGGED NAME" && frame.contains(shape.pos) =>
                {
                    Some((index, shape.pos))
                }
                _ => None,
            })
            .collect::<Vec<(usize, egui::Pos2)>>()
    };
    let offset = egui::vec2(30.0, 40.0);
    for (filter, floats) in [("", true), ("dragged", false)] {
        view.filter = filter.to_owned();
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let [(_, rest)] = names_in_list(&window, &shapes)[..] else {
            panic!("premise: the name painted once at rest (filter {filter:?})")
        };
        let press = egui::pos2(rest.x + 20.0, centre(row(&nodes, "03 CB DRAGGED NAME")).y);
        window.frame(pointer_at(press, true), |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
        let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(press + offset)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
        let painted = names_in_list(&window, &shapes);
        let [(index, at)] = painted.as_slice() else {
            panic!("the name painted once (filter {filter:?}), got {painted:?}")
        };
        if floats {
            let expected = rest + offset;
            assert!(
                at.distance(expected) <= 0.5,
                "the dragged name at its rest position plus the move: {at:?} vs {expected:?}"
            );
            let place_04 = nodes
                .iter()
                .find(|node| {
                    player_row(node) && node.label().is_some_and(|label| label.starts_with("04 "))
                })
                .and_then(accesskit::Node::bounds)
                .expect("row 04's bounds");
            let place = shapes
                .iter()
                .position(|clipped| match &clipped.shape {
                    egui::Shape::Text(shape) => {
                        shape.galley.job.text == "04"
                            && f64::from(shape.pos.y) >= place_04.y0
                            && f64::from(shape.pos.y) <= place_04.y1
                    }
                    _ => false,
                })
                .expect("row 04's place painted");
            assert!(
                *index > place,
                "the dragged name paints over the list, after row 04's place ({index} vs {place})"
            );
        } else {
            assert!(
                at.distance(rest) <= 0.5,
                "under a filter the name stays at rest: {at:?} vs {rest:?}"
            );
        }
        // Released where it was pressed: the drop moves nothing.
        window.frame(pointer_at(press, false), |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
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
