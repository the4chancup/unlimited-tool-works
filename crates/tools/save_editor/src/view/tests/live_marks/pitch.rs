use super::*;

/// A `FieldedNotA` violation's cross is an X over the starter's portrait
/// on the team view's Tactics pitch (the pitch is the field the
/// violation is about); with Live AATF off the marker carries none.
#[test]
fn a_fielded_violation_crosses_the_starters_portrait_on_the_pitch() {
    let (mut view, mut window) = open_734("live_marks_fielded");
    view.live_aatf = true;
    // Move a starter of preset 1's kick-off formation onto a position
    // his ratings do not hold at A, picked from his record (as `aatf`'s
    // own `FieldedNotA` fixture moves one to AMF).
    let index = 4;
    let mut team = team_entry(&view, 734);
    let slot = team.tactics.starting_eleven[index];
    let player_id = team.roster[usize::from(slot)].player_id;
    let position_byte = unrated_position_byte(&view, player_id);
    let kick_off = &mut team.tactics.presets[0].formations[0];
    assert_ne!(
        kick_off.players[index].position, position_byte,
        "premise: the move changes his spot"
    );
    kick_off.players[index].position = position_byte;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_team(1, team);

    // The check lands: a `FieldedNotA` on him in preset 1.
    live_checked(&mut window, &mut view, &[734]);
    let violations = view.live.check.result.as_ref().unwrap().1[0]
        .as_ref()
        .unwrap();
    let (fielded_slot, position) = violations
        .violations
        .iter()
        .find_map(|violation| {
            if let aatf::ViolationKind::FieldedNotA {
                preset: 1,
                position,
                ..
            } = violation.kind
            {
                Some((violation.slot, position))
            } else {
                None
            }
        })
        .expect("premise: a FieldedNotA on preset 1");
    assert_eq!(fielded_slot, Some(slot), "the moved starter's slot");
    assert_eq!(position.byte(), position_byte, "the moved-to position");

    // The team row selected (the pitch is the team view's): his
    // marker's portrait carries an X, corner to corner, its coloured
    // strokes in the error colour over black ones.
    view.selected = Some(Selection::Team(734));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let canvas = canvas_of(&nodes);
    let shown_slot = team_entry(&view, 734).tactics.presets[0].formations[0].players[index];
    let badge =
        egui::Rect::from_center_size(clamped_centre(canvas, shown_slot), egui::vec2(50.0, 14.0));
    let portrait = egui::Rect::from_min_max(
        egui::pos2(badge.left(), badge.top() - 50.0),
        egui::pos2(badge.right(), badge.top()),
    );
    let strokes = pitch_cross_strokes(&shapes);
    let [black, _, colour, _] = strokes.as_slice() else {
        panic!("expected one cross's four strokes, got {}", strokes.len())
    };
    assert_eq!(black.color, egui::Color32::BLACK, "the outline is black");
    assert_eq!(
        colour.color,
        window
            .ctx
            .style_of(window.ctx.theme())
            .visuals
            .error_fg_color,
        "the error colour"
    );
    assert!(
        strokes
            .iter()
            .all(|stroke| portrait.contains(stroke.points[0])
                && portrait.contains(stroke.points[1])
                && (stroke.points[0] - stroke.points[1]).x.abs() > 30.0),
        "the strokes cross his portrait {portrait:?}"
    );

    // Live AATF off: the pitch carries no cross.
    view.live_aatf = false;
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        pitch_cross_strokes(&shapes).is_empty(),
        "no cross on the pitch with Live AATF off"
    );
}

/// A painted line segment's ends, width and colour.
struct Stroke {
    points: [egui::Pos2; 2],
    color: egui::Color32,
}

/// The frame's line segments 5 or more wide: the pitch crosses' strokes
/// (the pitch's lines and corners are 2 wide), black outlines first.
fn pitch_cross_strokes(shapes: &[egui::epaint::ClippedShape]) -> Vec<Stroke> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke } if stroke.width >= 5.0 => Some(Stroke {
                points: *points,
                color: stroke.color,
            }),
            _ => None,
        })
        .collect()
}
