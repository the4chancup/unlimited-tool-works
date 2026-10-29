//! The geometry and paint tests: the screen's spot mapping and its snap,
//! the turf and the lines, the markers' badge, position, name, portrait
//! and corners, the rows' coloured names.

use pes_version::PesVersion;

use super::*;
use crate::tactics::{FieldedMark, Medal};
use crate::test_support::{
    Headless, centre, entry, fixture_rows, fixture_tactics, painted_color, pointer_at, roster_rows,
};
use crate::violations::CROSS;

/// An outfield slot at (`x`, `y`), for the pure mapping tests.
fn outfield(x: u8, y: u8) -> FormationSlot {
    FormationSlot { position: 1, x, y }
}

// The formation screen's own slot-to-spot mapping: x 0 to 104 across
// the canvas' width, y 24 the halfway line, the attack end
// up; the keeper sits on the goal area's line whatever his stored spot.
#[test]
fn the_badge_centre_is_the_screens_spot_mapping() {
    let origin = egui::pos2(40.0, 80.0);
    near(
        badge_centre(origin, outfield(52, 24)),
        origin + egui::vec2(159.8, 200.8),
        0.05,
    );
    near(
        badge_centre(origin, outfield(0, 0)),
        origin + egui::vec2(12.8, 363.8),
        0.05,
    );
    near(
        badge_centre(origin, outfield(104, 48)),
        origin + egui::vec2(306.8, 37.7),
        0.05,
    );
    for (x, y) in [(52, 3), (10, 40)] {
        near(
            badge_centre(origin, FormationSlot { position: 0, x, y }),
            origin + egui::vec2(160.0, 358.5),
            0.05,
        );
    }
}

// `snap` is the mapping's inverse: every stored (x, y) round-trips, and
// a point past each edge clamps to it.
#[test]
fn snap_inverts_the_mapping_and_clamps_to_the_grid() {
    let origin = egui::pos2(40.0, 80.0);
    for x in 0..=GRID_X {
        for y in 0..=GRID_Y {
            let slot = outfield(x, y);
            assert_eq!(
                snap(badge_centre(origin, slot), origin),
                (x, y),
                "({x}, {y})"
            );
        }
    }
    assert_eq!(
        snap(origin + egui::vec2(500.0, 200.0), origin),
        (104, 24),
        "past the right edge"
    );
    assert_eq!(
        snap(origin + egui::vec2(160.0, 500.0), origin),
        (52, 0),
        "past the near edge"
    );
    assert_eq!(
        snap(origin + egui::vec2(-50.0, -50.0), origin),
        (0, 48),
        "past the far corner"
    );
}

// The marker's whole box: the badge's width, the portrait's top to the
// name's bottom, the badge centre clamped so it stays inside the canvas.
#[test]
fn the_marker_box_centres_on_the_badge_and_stays_inside() {
    let origin = egui::pos2(40.0, 80.0);
    let canvas = egui::Rect::from_min_size(origin, PITCH);
    let rect = marker_rect(badge_centre(origin, outfield(52, 24)), canvas);
    near(rect.min, origin + egui::vec2(134.8, 143.8), 0.05);
    near(rect.max, origin + egui::vec2(184.8, 220.8), 0.05);
    assert_eq!(
        marker_rect(origin + egui::vec2(-30.0, 200.0), canvas).left(),
        canvas.left(),
        "a centre past the left edge clamps to it"
    );
    assert_eq!(
        marker_rect(origin + egui::vec2(350.0, 200.0), canvas).right(),
        canvas.right(),
        "a centre past the right edge clamps to it"
    );
    assert_eq!(
        marker_rect(origin + egui::vec2(160.0, -30.0), canvas).top(),
        canvas.top(),
        "a centre past the top clamps to it"
    );
    assert_eq!(
        marker_rect(origin + egui::vec2(160.0, 410.0), canvas).bottom(),
        canvas.bottom(),
        "a centre past the bottom clamps to it"
    );
}

// The turf: the game texture's own size, eleven horizontal bands dark at
// both ends over their two greens, the grain a fixed noise of luma
// deviation about 3.5, deterministic.
#[test]
fn the_turf_is_the_game_textures_bands_with_its_grain() {
    let image = turf_image(&Palette::DARK);
    assert_eq!(image.size, TURF_SIZE);
    // A row's or band's mean channel.
    let mean = |rows: &[usize], channel: usize| {
        let (mut sum, mut n) = (0.0f64, 0usize);
        for &y in rows {
            for x in 0..image.width() {
                sum += f64::from(image.pixels[y * image.width() + x][channel]);
                n += 1;
            }
        }
        sum / n as f64
    };
    let rows_of = |band: usize| {
        (0..image.height())
            .filter(|&y| y * TURF_BANDS / image.height() == band)
            .collect::<Vec<usize>>()
    };
    let channels = |colour: egui::Color32| [colour.r(), colour.g(), colour.b()];
    let dark = channels(Palette::DARK.turf_dark_band);
    let light = channels(Palette::DARK.turf_light_band);
    for band in 0..TURF_BANDS {
        let base = if band % 2 == 0 { dark } else { light };
        for (channel, &base) in base.iter().enumerate() {
            assert!(
                (mean(&rows_of(band), channel) - f64::from(base)).abs() <= 1.0,
                "band {band} channel {channel}"
            );
        }
    }
    // Bands 0 and 10 are dark (both ends), band 5 light.
    for (row, base) in [(0usize, dark), (575, dark), (288, light)] {
        for (channel, &base) in base.iter().enumerate() {
            assert!(
                (mean(&[row], channel) - f64::from(base)).abs() <= 1.0,
                "row {row} channel {channel}"
            );
        }
    }
    // The grain, away from band 1's edges.
    let lumas: Vec<f64> = (53..=103)
        .flat_map(|y| (0..image.width()).map(move |x| (y, x)))
        .map(|(y, x)| f64::from(image.pixels[y * image.width() + x].intensity()) * 255.0)
        .collect();
    let mean_luma = lumas.iter().sum::<f64>() / lumas.len() as f64;
    let std =
        (lumas.iter().map(|l| (l - mean_luma).powi(2)).sum::<f64>() / lumas.len() as f64).sqrt();
    assert!(
        (3.0..=4.0).contains(&std),
        "the grain's luma deviation {std}"
    );
    assert_eq!(
        turf_image(&Palette::DARK).pixels,
        image.pixels,
        "deterministic"
    );
}

// The grain itself, pixel-exact on row 0's dark band: the fixed-seed
// xorshift32's known answer.
#[test]
fn the_turf_grain_is_the_fixed_seed_xorshift() {
    let image = turf_image(&Palette::DARK);
    for (x, expected) in [
        egui::Color32::from_rgb(43, 85, 27),
        egui::Color32::from_rgb(50, 92, 34),
        egui::Color32::from_rgb(45, 87, 29),
        egui::Color32::from_rgb(39, 81, 23),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(image.pixels[x], expected, "pixel {x}");
    }
}

// The lines, read off the painted shapes in canvas-relative
// coordinates: the halfway line, the centre circle,
// the two penalty boxes and goal areas as segments, the two arcs as
// paths; every stroke white at 20%, 2 px wide.
#[test]
fn the_painted_lines_are_the_screens_markings() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);

    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let at = |x: f32, y: f32| canvas.min + egui::vec2(x, y);
    let mut segments: Vec<egui::Rect> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == Palette::DARK.pitch_lines =>
            {
                assert_eq!(stroke.width, 2.0, "every line 2 px");
                Some(egui::Rect::from_two_pos(points[0], points[1]))
            }
            _ => None,
        })
        .collect();
    segments.sort_by(|a, b| {
        (a.min.y, a.min.x, a.max.y, a.max.x)
            .partial_cmp(&(b.min.y, b.min.x, b.max.y, b.max.x))
            .expect("ordered bounds")
    });
    let rect = |a: (f32, f32), b: (f32, f32)| egui::Rect::from_two_pos(at(a.0, a.1), at(b.0, b.1));
    assert_eq!(
        segments,
        [
            rect((80.0, 0.0), (80.0, 66.0)),
            rect((123.0, 0.0), (123.0, 22.0)),
            rect((197.0, 0.0), (197.0, 22.0)),
            rect((240.0, 0.0), (240.0, 66.0)),
            rect((123.0, 22.0), (197.0, 22.0)),
            rect((80.0, 66.0), (240.0, 66.0)),
            rect((0.0, 192.0), (320.0, 192.0)),
            rect((80.0, 318.0), (240.0, 318.0)),
            rect((80.0, 318.0), (80.0, 384.0)),
            rect((240.0, 318.0), (240.0, 384.0)),
            rect((123.0, 362.0), (197.0, 362.0)),
            rect((123.0, 362.0), (123.0, 384.0)),
            rect((197.0, 362.0), (197.0, 384.0)),
        ],
        "the halfway line and the twelve box segments"
    );
    let circles: Vec<(egui::Pos2, f32)> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Circle(shape) if shape.stroke.color == Palette::DARK.pitch_lines => {
                assert_eq!(shape.stroke.width, 2.0);
                Some((shape.center, shape.radius))
            }
            _ => None,
        })
        .collect();
    assert_eq!(circles, [(at(160.0, 192.0), 36.0)], "the centre circle");
    let mut arcs: Vec<Vec<egui::Pos2>> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Path(shape)
                if matches!(
                    shape.stroke.color,
                    egui::epaint::ColorMode::Solid(color) if color == Palette::DARK.pitch_lines
                ) =>
            {
                assert_eq!(shape.stroke.width, 2.0);
                assert!(!shape.closed, "the arc is an open path");
                Some(shape.points.clone())
            }
            _ => None,
        })
        .collect();
    arcs.sort_by(|a, b| a[0].y.partial_cmp(&b[0].y).expect("ordered arcs"));
    assert_eq!(arcs.len(), 2, "one arc per end");
    // Each arc: 17 points on the centre circle's radius about the spot
    // 44 from its goal line, only the part beyond the box's far edge.
    for (points, spot, top) in [
        (arcs[0].as_slice(), at(160.0, 44.0), true),
        (arcs[1].as_slice(), at(160.0, 340.0), false),
    ] {
        assert_eq!(points.len(), 17, "the arc's points");
        for point in points {
            assert!(
                (point.distance(spot) - 36.0).abs() <= 0.01,
                "{point:?} on the circle"
            );
            assert!(
                if top {
                    point.y >= spot.y + 22.0 - 0.01
                } else {
                    point.y <= spot.y - 22.0 + 0.01
                },
                "{point:?} beyond the box's far edge"
            );
        }
    }
}

// The badge's position-text colours are the game's groups: forwards,
// midfielders, defenders and the keeper in their own colours, a
// position past the table white.
#[test]
fn the_position_colours_follow_the_game_s_groups() {
    assert_eq!(
        position_colour(&Palette::DARK, 0),
        egui::Color32::from_rgb(0xff, 0xde, 0x00),
        "GK"
    );
    for position in 1..=3 {
        assert_eq!(
            position_colour(&Palette::DARK, position),
            egui::Color32::from_rgb(0x00, 0xd2, 0xff),
            "{position} a defender"
        );
    }
    for position in 4..=8 {
        assert_eq!(
            position_colour(&Palette::DARK, position),
            egui::Color32::from_rgb(0x57, 0xe4, 0x2b),
            "{position} a midfielder"
        );
    }
    for position in 9..=12 {
        assert_eq!(
            position_colour(&Palette::DARK, position),
            egui::Color32::from_rgb(0xff, 0x2c, 0x77),
            "{position} a forward"
        );
    }
    assert_eq!(
        position_colour(&Palette::DARK, 13),
        egui::Color32::WHITE,
        "past the table"
    );
}

// The markers on the fixture's formation: a 50 x 14 badge of `#242b2d`
// centred on the slot's clamped centre (the keeper's the
// screen's spot), the position's abbreviation in its group's colour, the
// captain's mark on his marker alone, a row's portrait drawn when the
// host hands one.
#[test]
fn the_markers_paint_badge_position_captain_and_portrait() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    let texture = window.ctx.load_texture(
        "a portrait",
        egui::ColorImage::new([4, 4], vec![egui::Color32::RED; 16]),
        egui::TextureOptions::LINEAR,
    );
    // A non-identity eleven: a portrait or captaincy looked up by the
    // marker's index rather than its roster slot would look right on
    // the fixture's [0, 1, ...].
    tactics.starting_eleven.swap(4, 9);
    tactics.starting_eleven.swap(2, 8);
    assert_ne!(
        tactics.starting_eleven[4], 4,
        "premise: the portrait's slot and index differ"
    );
    assert_ne!(
        tactics.starting_eleven[2], 2,
        "premise: the captain's slot and index differ"
    );
    let portrait_slot = usize::from(tactics.starting_eleven[4]);
    roster[portrait_slot].portrait = Some(texture.id());
    tactics.set_pieces.captain = tactics.starting_eleven[2];
    let players = tactics.presets[0].formations[0].players;
    let keeper = players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("the fixture has a keeper slot");

    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    assert_eq!(
        centres[keeper],
        canvas.min + egui::vec2(160.0, 358.5),
        "the keeper sits at the screen's keeper spot"
    );
    for (index, slot) in players.iter().enumerate() {
        let badge = egui::Rect::from_center_size(centres[index], egui::vec2(50.0, 14.0));
        let rect = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) if shape.rect == badge => Some(shape),
                _ => None,
            })
            .next()
            .unwrap_or_else(|| panic!("no badge for marker {index}"));
        assert_eq!(
            rect.fill,
            egui::Color32::from_rgb(0x24, 0x2b, 0x2d),
            "marker {index}'s badge"
        );
        // The position's abbreviation at the badge's left, in the
        // position's group's colour.
        let label = position_label(slot.position);
        let text = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape)
                    if shape.galley.job.text == label
                        && (shape.pos.x - (badge.left() + 2.0)).abs() <= 0.01
                        && (shape.visual_bounding_rect().center().y - badge.center().y).abs()
                            <= 2.0 =>
                {
                    Some(shape)
                }
                _ => None,
            })
            .next()
            .unwrap_or_else(|| panic!("no position text for marker {index}"));
        assert_eq!(
            painted_color(text),
            position_colour(&Palette::DARK, slot.position),
            "marker {index}'s {label} in its group's colour"
        );
    }
    // The captain's mark on the captain's marker alone: the white box
    // and the badge-coloured outline under it.
    let badge2 = egui::Rect::from_center_size(centres[2], egui::vec2(50.0, 14.0));
    let marks = |fill: egui::Color32, size: egui::Vec2| {
        shapes
            .iter()
            .enumerate()
            .filter_map(|(index, clipped)| match &clipped.shape {
                egui::Shape::Rect(shape) if shape.fill == fill && shape.rect.size() == size => {
                    Some((index, shape.rect))
                }
                _ => None,
            })
            .collect::<Vec<(usize, egui::Rect)>>()
    };
    let outlines = marks(Palette::DARK.badge_fill, egui::vec2(14.0, 13.0));
    let [(outline_at, outline)] = outlines.as_slice() else {
        panic!("the badge-coloured outline above the captain's badge alone")
    };
    assert_eq!(
        *outline,
        egui::Rect::from_min_max(
            egui::pos2(badge2.left() + 1.0, badge2.top() - 17.0),
            egui::pos2(badge2.left() + 15.0, badge2.top() - 4.0),
        ),
        "the outline a pixel out of the white box"
    );
    let whites = marks(egui::Color32::WHITE, egui::vec2(12.0, 11.0));
    let [(white_at, white)] = whites.as_slice() else {
        panic!("the white mark above the captain's badge alone")
    };
    assert_eq!(
        *white,
        egui::Rect::from_min_max(
            egui::pos2(badge2.left() + 2.0, badge2.top() - 16.0),
            egui::pos2(badge2.left() + 14.0, badge2.top() - 5.0),
        ),
        "the white box"
    );
    assert!(
        outline_at < white_at,
        "the outline paints before the white box"
    );
    // The portrait: the whole texture over the badge of the row that
    // hands one, no other image but the turf.
    let images: Vec<&egui::epaint::ClippedShape> = shapes
        .iter()
        .filter(|clipped| matches!(clipped.shape, egui::Shape::Mesh(_)))
        .collect();
    assert_eq!(images.len(), 2, "the turf and the one portrait");
    let portrait = images
        .iter()
        .find(|clipped| clipped.shape.texture_id() == texture.id())
        .expect("the portrait's image shape");
    let badge4 = egui::Rect::from_center_size(centres[4], egui::vec2(50.0, 14.0));
    assert_eq!(
        portrait.shape.visual_bounding_rect(),
        egui::Rect::from_min_size(
            egui::pos2(badge4.left(), badge4.top() - 50.0),
            egui::vec2(50.0, 50.0),
        ),
        "the 50 x 50 portrait standing on the badge"
    );
}

// The name below the badge is drawn in its colour codes' colours, white
// where it has none.
#[test]
fn a_markers_name_keeps_its_colour_codes_colours() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    rows[usize::from(tactics.starting_eleven[4])].1 = "\u{11}cff0000ffRED\u{11}d NAME".to_owned();
    let roster = roster_rows(&rows);

    let (_, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let galley = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == "RED NAME" => {
                Some(shape.galley.job.clone())
            }
            _ => None,
        })
        .next()
        .expect("the marker's name galley");
    let colours: Vec<egui::Color32> = galley
        .sections
        .iter()
        .map(|section| section.format.color)
        .collect();
    assert_eq!(
        colours,
        [egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE,],
        "the coded run red, the plain run white"
    );
}

// The formation screen's selection marks, reused for the editor's
// states: the open popover's marker gets the selected corners, no other
// does. Every marker gets the candidates' while an assignment mode is
// armed, and a join member's badge is `Palette::DARK.join_fill` under the armed join
// mode.
#[test]
fn the_corners_and_the_join_fill_mark_the_states() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;
    let index = 3;
    // A non-identity eleven: a join fill looked up by the marker's index
    // rather than its roster slot would look right on the fixture's
    // [0, 1, ...].
    tactics.starting_eleven.swap(index, 9);
    assert_ne!(
        tactics.starting_eleven[index], index as u8,
        "premise: the joining player's slot and index differ"
    );

    // The open popover's marker.
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    window.click_pitch(
        marker_point(&nodes, &players, index),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert_eq!(view.popover.map(|open| open.slot), Some(index));
    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let badge = egui::Rect::from_center_size(centres[index], egui::vec2(50.0, 14.0));
    let portrait_box = egui::Rect::from_min_size(
        egui::pos2(badge.left(), badge.top() - 50.0),
        egui::vec2(50.0, 50.0),
    );
    let corners = |shapes: &[egui::epaint::ClippedShape], colour: egui::Color32| {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::LineSegment { points, stroke } if stroke.color == colour => {
                    Some(*points)
                }
                _ => None,
            })
            .collect::<Vec<[egui::Pos2; 2]>>()
    };
    // Each of `segments` is a 9 px arm, one end a `portrait_box` corner.
    let assert_corner_arms = |segments: &[[egui::Pos2; 2]], portrait_box: egui::Rect| {
        let box_corners = [
            portrait_box.left_top(),
            portrait_box.right_top(),
            portrait_box.left_bottom(),
            portrait_box.right_bottom(),
        ];
        for points in segments {
            assert!(
                (points[0].distance(points[1]) - 9.0).abs() <= 0.01,
                "a 9 px arm"
            );
            assert!(
                box_corners.iter().any(|corner| points.contains(corner)),
                "{points:?} ends on a portrait-box corner"
            );
        }
    };
    let selected = corners(&shapes, Palette::DARK.selected_corners);
    assert_eq!(selected.len(), 8, "the open marker's four corner arms");
    assert_corner_arms(&selected, portrait_box);
    for points in selected {
        for point in points {
            assert!(
                portrait_box.expand(1.0).contains(point),
                "{point:?} on the marker's portrait box"
            );
        }
    }
    assert_eq!(
        corners(&shapes, Palette::DARK.candidate_corners).len(),
        0,
        "no candidate corners with nothing armed"
    );

    // An armed mode: the candidates' corners on every marker (marker
    // `index`'s own checked the same way, the box read off the new
    // frame's canvas).
    view.popover = None;
    view.assign = Some(Assign::Role(5));
    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let candidates = corners(&shapes, Palette::DARK.candidate_corners);
    assert_eq!(
        candidates.len(),
        88,
        "the candidates' corners on all eleven"
    );
    assert_eq!(corners(&shapes, Palette::DARK.selected_corners).len(), 0);
    let canvas = canvas_rect(&nodes);
    let badge =
        egui::Rect::from_center_size(centres_of(canvas, &players)[index], egui::vec2(50.0, 14.0));
    let portrait_box = egui::Rect::from_min_size(
        egui::pos2(badge.left(), badge.top() - 50.0),
        egui::vec2(50.0, 50.0),
    );
    let own: Vec<[egui::Pos2; 2]> = candidates
        .iter()
        .copied()
        .filter(|points| {
            [
                portrait_box.left_top(),
                portrait_box.right_top(),
                portrait_box.left_bottom(),
                portrait_box.right_bottom(),
            ]
            .iter()
            .any(|corner| points.contains(corner))
        })
        .collect();
    assert_eq!(own.len(), 8, "the marker's own four corner arms");
    assert_corner_arms(&own, portrait_box);

    // Join armed: a member's badge fills Palette::DARK.join_fill, a non-member's
    // stays Palette::DARK.badge_fill.
    view.assign = Some(Assign::Join);
    tactics.players_to_join_attack = [tactics.starting_eleven[3], u8::MAX, u8::MAX];
    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let fill_of = |index: usize| {
        let badge = egui::Rect::from_center_size(centres[index], egui::vec2(50.0, 14.0));
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) if shape.rect == badge => Some(shape.fill),
                _ => None,
            })
            .next()
            .unwrap_or_else(|| panic!("no badge for marker {index}"))
    };
    assert_eq!(fill_of(3), Palette::DARK.join_fill, "the member's badge");
    assert_eq!(fill_of(4), Palette::DARK.badge_fill, "a non-member's badge");

    // Nothing armed: no corners at all.
    view.assign = None;
    let (_, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_eq!(corners(&shapes, Palette::DARK.candidate_corners).len(), 0);
    assert_eq!(corners(&shapes, Palette::DARK.selected_corners).len(), 0);
}

// The markers are drawn from the top of the canvas down, and a press or
// a drop hits the marker drawn on top: of two slots on top of each
// other, the lower one's badge paints after the upper's and a press
// inside both boxes opens the lower one's popover.
#[test]
fn the_lower_marker_paints_last_and_takes_the_press() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let (a, b) = (5usize, 3usize);
    // A above (y 20), B below (y 17) on the same x; B's lower index makes
    // an index-order paint put it under A, so only the draw order wins.
    assert!(
        tactics.presets[0].formations[0].players[a].position != 0
            && tactics.presets[0].formations[0].players[b].position != 0,
        "premise: two outfield slots"
    );
    tactics.presets[0].formations[0].players[a].x = 52;
    tactics.presets[0].formations[0].players[a].y = 20;
    tactics.presets[0].formations[0].players[b].x = 52;
    tactics.presets[0].formations[0].players[b].y = 17;
    let players = tactics.presets[0].formations[0].players;

    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let badge_index = |index: usize| {
        let badge = egui::Rect::from_center_size(centres[index], egui::vec2(50.0, 14.0));
        shapes
            .iter()
            .position(
                |clipped| matches!(&clipped.shape, egui::Shape::Rect(shape) if shape.rect == badge),
            )
            .unwrap_or_else(|| panic!("no badge for marker {index}"))
    };
    assert!(
        badge_index(b) > badge_index(a),
        "the lower marker's badge paints after the upper's"
    );
    // A point inside both boxes: A's badge centre, 20 px above B's.
    assert!(
        marker_rect(centres[b], canvas).contains(centres[a]),
        "premise: the press is inside both boxes"
    );
    window.click_pitch(
        centres[a],
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(b),
        "the press hits the marker drawn on top"
    );
}

/// The frame's badge rects (`Palette::DARK.badge_fill`, badge-sized) in paint order.
pub(super) fn badge_rects(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::Rect> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == Palette::DARK.badge_fill
                    && (shape.rect.width() - BADGE_W).abs() <= 0.5
                    && (shape.rect.height() - BADGE_H).abs() <= 0.5 =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .collect()
}

/// `a` is off `b`, the negation of `near`'s per-coordinate 0.5.
fn off(a: egui::Pos2, b: egui::Pos2) -> bool {
    (a.x - b.x).abs() > 0.5 || (a.y - b.y).abs() > 0.5
}

// While a marker is dragged he paints last, his badge centred on the
// spot under the pointer less the press's offset — eleven badges as
// ever, the last where a release would put him and none left at his own
// centre.
#[test]
fn a_dragged_marker_paints_last_at_the_pointer() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let order = draw_order(&centres);
    // `index` is an outfield marker painted after the first outfield in
    // the order (the first is what a dragged lookup without its
    // `dragged` half would find); `onto` is a grass point — over a
    // marker the drag would dim him for the swap instead.
    let first_outfield = order
        .iter()
        .copied()
        .find(|&i| players[i].position != 0)
        .expect("the formation has outfield slots");
    let index = order
        .iter()
        .copied()
        .find(|&i| players[i].position != 0 && i != first_outfield)
        .expect("a second outfield slot");
    let onto = grass_point(canvas, &players, egui::Rect::NOTHING);
    let from = marker_point(&nodes, &players, index);
    assert!(
        off(
            held_centre(canvas, players[index], from, onto),
            centres[index]
        ),
        "premise: the drop point is off his own centre"
    );

    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(from, true),
    );
    let (_, shapes) = window.frame(vec![egui::Event::PointerMoved(onto)], |ui| {
        pitch(
            ui,
            "pitch",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    let badges = badge_rects(&shapes);
    assert_eq!(badges.len(), 11, "eleven badges, the dragged one's moved");
    near(
        badges.last().expect("eleven badges").center(),
        held_centre(canvas, players[index], from, onto),
        0.5,
    );
    assert!(
        badges
            .iter()
            .all(|badge| off(badge.center(), centres[index])),
        "no badge left at the dragged marker's own centre"
    );
}

// On the release frame the dropped marker already paints at the snapped
// drop point (a drop on the grass reaches it at once): his badge's
// centre is the written slot's, not his pre-drag spot's.
#[test]
fn the_released_marker_paints_at_the_drop_point() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;
    let index = 4;

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let from = marker_point(&nodes, &players, index);
    let onto = grass_point(canvas, &players, egui::Rect::NOTHING);
    let (x, y) = snap(onto, canvas.min);
    assert_ne!(
        (players[index].x, players[index].y),
        (x, y),
        "premise: a real move"
    );

    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(from, true),
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(onto)],
    );
    let (_, shapes) = window.frame(pointer_at(onto, false), |ui| {
        pitch(
            ui,
            "pitch",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    let dropped = clamp_centre(
        badge_centre(canvas.min, tactics.presets[0].formations[0].players[index]),
        canvas,
    );
    let badges = badge_rects(&shapes);
    assert_eq!(badges.len(), 11, "eleven badges");
    assert!(
        badges.iter().any(|badge| !off(badge.center(), dropped)),
        "his badge at the drop's snapped centre"
    );
    assert!(
        badges
            .iter()
            .all(|badge| off(badge.center(), centres[index])),
        "none left at his pre-drag spot"
    );
}

// The keeper's slot is fixed: pressed and dragged, his marker paints at
// his own spot — the dragged repaint skips him — and no badge sits at
// the pointer.
#[test]
fn a_pressed_keeper_paints_at_his_spot() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;
    let keeper = players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("the fixture has a keeper slot");

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let point = marker_point(&nodes, &players, keeper);
    assert!(
        centres
            .iter()
            .enumerate()
            .all(|(i, _)| i == keeper || !marker_rect(centres[i], canvas).contains(point)),
        "premise: no other marker covers the press"
    );
    // A grass point off every marker, whose clamped centre is off every
    // marker's centre too.
    let onto = (0..=GRID_X)
        .flat_map(|x| (0..=GRID_Y).map(move |y| (x, y)))
        .map(|(x, y)| badge_centre(canvas.min, outfield(x, y)))
        .find(|point| {
            centres
                .iter()
                .all(|centre| !marker_rect(*centre, canvas).contains(*point))
                && centres
                    .iter()
                    .all(|centre| off(*centre, clamp_centre(*point, canvas)))
        })
        .expect("a grass point off every marker and centre");

    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(point, true),
    );
    let (_, shapes) = window.frame(vec![egui::Event::PointerMoved(onto)], |ui| {
        pitch(
            ui,
            "pitch",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    let badges = badge_rects(&shapes);
    assert_eq!(badges.len(), 11, "eleven badges");
    assert!(
        badges.iter().any(|badge| {
            let c = badge.center();
            (c.x - (canvas.min.x + KEEPER_X)).abs() <= 0.5
                && (c.y - (canvas.min.y + KEEPER_Y)).abs() <= 0.5
        }),
        "his badge still centred on his spot"
    );
    assert!(
        badges
            .iter()
            .all(|badge| off(badge.center(), clamp_centre(onto, canvas))),
        "no badge at the pointer"
    );
}

// `name_fits`: a name no wider than the badge is centred and never
// moves; a hair wider scrolls.
#[test]
fn a_name_as_wide_as_the_badge_fits() {
    assert!(name_fits(50.0), "the badge's own width fits");
    assert!(!name_fits(50.01), "a hair wider scrolls");
    assert!(name_fits(12.0), "a narrow one fits");
}

// `name_scroll`, the scroll measured on the game's screen: a wide name
// holds still `NAME_PERIOD` less its `width + NAME_GAP` loop at
// `NAME_SPEED`, then moves one loop a period, ending where it began.
#[test]
fn name_scroll_rolls_one_loop_a_period() {
    for (seconds, want) in [
        (0.0, 0.0),
        (10.4, 0.0),
        (11.5, 40.0),
        (13.4, 116.0),
        (13.5, 0.0),
        (25.0, 40.0),
    ] {
        assert!(
            (name_scroll(104.0, seconds) - want).abs() <= 1e-3,
            "{seconds} s"
        );
    }
}

// `name_still_for` answers how long until the name next moves: the rest
// of its still phase, 0 while it moves.
#[test]
fn name_still_for_says_when_it_next_moves() {
    for (seconds, want) in [(0.0, 10.5), (11.0, 0.0), (13.5, 10.5)] {
        assert!(
            (name_still_for(104.0, seconds) - want).abs() <= 1e-3,
            "{seconds} s"
        );
    }
}

// The frame's text shapes with the galley `text`, in `pos.x` order.
fn name_galleys<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    text: &str,
) -> Vec<&'a egui::epaint::TextShape> {
    let mut galleys: Vec<&egui::epaint::TextShape> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == text => Some(shape),
            _ => None,
        })
        .collect();
    galleys.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
    galleys
}

// A name wider than its badge scrolls as the game's does: two copies a
// loop apart, the first `name_scroll` left of the badge's left edge,
// clocked from the pitch's first frame; a name that fits stays centred,
// unmoved.
#[test]
fn a_wide_name_scrolls_two_copies_from_the_pitchs_first_frame() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    let index = 4;
    rows[usize::from(tactics.starting_eleven[index])].1 = "3DSMAX HAS STOPPED WORKING".to_owned();
    rows[usize::from(tactics.starting_eleven[3])].1 = "SHORT".to_owned();
    let roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;

    // The clock starts at the pitch's first frame, 100.0.
    window.pitch_paint_at(100.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (nodes, shapes) =
        window.pitch_paint_at(111.5, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let badge_left = centres[index].x - 25.0;
    let galleys = name_galleys(&shapes, "3DSMAX HAS STOPPED WORKING");
    assert_eq!(galleys.len(), 2, "the name and its repeat a loop apart");
    let width = galleys[0].galley.size().x;
    assert!(width > 50.0, "premise: the name is wider than the badge");
    assert!(
        (galleys[0].pos.x - (badge_left - name_scroll(width, 11.5))).abs() <= 0.5,
        "the first copy scrolled left of the badge"
    );
    assert!(
        (galleys[1].pos.x - (galleys[0].pos.x + width + NAME_GAP)).abs() <= 0.5,
        "its repeat a loop to the right"
    );

    // A name that fits stays centred on its badge, unmoved.
    let short_galleys = name_galleys(&shapes, "SHORT");
    assert_eq!(short_galleys.len(), 1, "a fitting name paints once");
    let short = short_galleys[0];
    assert!(
        (short.pos.x - (centres[3].x - short.galley.size().x / 2.0)).abs() <= 0.5
            && (short.pos.y - (centres[3].y + NAME_DY - short.galley.size().y / 2.0)).abs() <= 0.5,
        "the short name still centred under its badge"
    );
}

// The clock restarts when a frame passed without the pitch: a frame at
// 105 without it and the 111.5 frame's offset is the fresh start's 0.
#[test]
fn the_scrolls_clock_restarts_when_a_frame_skipped_the_pitch() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    let index = 4;
    rows[usize::from(tactics.starting_eleven[index])].1 = "3DSMAX HAS STOPPED WORKING".to_owned();
    let roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;

    window.pitch_paint_at(100.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    window.frame_at(Some(105.0), vec![], |_ui| {});
    let (nodes, shapes) =
        window.pitch_paint_at(111.5, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let badge_left = centres[index].x - 25.0;
    let galleys = name_galleys(&shapes, "3DSMAX HAS STOPPED WORKING");
    assert_eq!(galleys.len(), 2, "the name and its repeat a loop apart");
    assert!(
        (galleys[0].pos.x - badge_left).abs() <= 0.5,
        "the restarted clock's offset is 0"
    );
}

// The same frame can run the pitch more than once when a widget asks
// for a discard pass (a `Grid`'s, a `Resize`'s sizing pass):
// `cumulative_frame_nr` counts frames, not passes, so the second pass
// meets `last == frame_nr` and must keep `started`, not restart it.
#[test]
fn the_scrolls_clock_survives_a_discarded_pass() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    let index = 4;
    rows[usize::from(tactics.starting_eleven[index])].1 = "3DSMAX HAS STOPPED WORKING".to_owned();
    let roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;

    // The clock starts at the pitch's first frame, 100.0.
    window.pitch_paint_at(100.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let passes_before = window.ctx.cumulative_pass_nr();
    let (nodes, shapes) = window.frame_at(Some(111.5), vec![], |ui| {
        if ui.ctx().current_pass_index() == 0 {
            ui.ctx().request_discard("test");
        }
        pitch(
            ui,
            "pitch",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    assert_eq!(
        window.ctx.cumulative_pass_nr() - passes_before,
        2,
        "premise: the frame ran two passes"
    );
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let badge_left = centres[index].x - 25.0;
    let galleys = name_galleys(&shapes, "3DSMAX HAS STOPPED WORKING");
    assert_eq!(galleys.len(), 2, "the name and its repeat a loop apart");
    let width = galleys[0].galley.size().x;
    assert!(
        (galleys[0].pos.x - (badge_left - name_scroll(width, 11.5))).abs() <= 0.5,
        "the second pass kept the clock's start"
    );
}

// While a shown name is wide the pitch asks a repaint when it next
// moves: its still time while still, at once while it moves; with no
// wide name it asks nothing. (egui forces the context's first frame and
// every frame after an immediate repaint to delay 0, so the still
// measure runs on the third pitch frame and the no-request check on the
// frame after the carry-over drains.)
#[test]
fn the_pitch_repaints_when_the_next_name_moves() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    rows[usize::from(tactics.starting_eleven[4])].1 = "3DSMAX HAS STOPPED WORKING".to_owned();
    let roster = roster_rows(&rows);

    // The clock's first frame, at 100.0; the next drains the carry-over
    // frame-1 repaint egui always makes (the texture and font uploads).
    window.pitch_paint_at(100.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    window.pitch_paint_at(105.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (_, shapes) =
        window.pitch_paint_at(105.1, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let width = name_galleys(&shapes, "3DSMAX HAS STOPPED WORKING")[0]
        .galley
        .size()
        .x;
    let still = name_still_for(width, 5.1);
    assert!(
        (window.repaint_delay().as_secs_f32() - still).abs() <= 0.05,
        "the still name's repaint in its still time ({} vs {still})",
        window.repaint_delay().as_secs_f32()
    );

    // 12 s in, the name moves: the repaint is at once.
    window.pitch_paint_at(112.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert!(
        window.repaint_delay().as_secs_f32() <= 0.05,
        "the moving name's repaint at once ({:?})",
        window.repaint_delay()
    );

    // No wide name (the fixture's own names are all wide too): the
    // pitch requests nothing (the immediate request's carry-over frame
    // drains first).
    let mut rows = fixture_rows(PesVersion::Pes21);
    for slot in tactics.starting_eleven {
        rows[usize::from(slot)].1 = "P".to_owned();
    }
    let roster = roster_rows(&rows);
    window.pitch_paint_at(113.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    window.pitch_paint_at(114.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_eq!(
        window.repaint_delay(),
        std::time::Duration::MAX,
        "nothing requested without a wide name"
    );
}

// `TacticsView::scroll_names` (the formation bar's checkbox): on, a
// name wider than its badge moves as the game's; off, every wide name is
// still at its badge's left — the scroll's resting phase, no repaint to
// ask for either.
#[test]
fn scroll_names_off_holds_wide_names_still_at_the_badges_left() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    let index = 4;
    rows[usize::from(tactics.starting_eleven[index])].1 = "3DSMAX HAS STOPPED WORKING".to_owned();
    let roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;
    let x_of = |shapes: &[egui::epaint::ClippedShape]| {
        name_galleys(shapes, "3DSMAX HAS STOPPED WORKING")[0].pos.x
    };

    // The clock starts at the pitch's first frame, 100.0; the name moves
    // in the last seconds of its 13.5 s period.
    window.pitch_paint_at(100.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (nodes, moving_a) =
        window.pitch_paint_at(111.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (_, moving_b) =
        window.pitch_paint_at(112.5, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_ne!(
        x_of(&moving_a),
        x_of(&moving_b),
        "the wide name moves between frames while names scroll"
    );
    let canvas = canvas_rect(&nodes);
    let badge_left = centres_of(canvas, &players)[index].x - 25.0;

    view.scroll_names = false;
    let (_, still_a) =
        window.pitch_paint_at(111.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (_, still_b) =
        window.pitch_paint_at(112.5, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_eq!(
        x_of(&still_a),
        x_of(&still_b),
        "the same frames hold the name still with Scroll names off"
    );
    assert!(
        (x_of(&still_a) - badge_left).abs() <= 0.5,
        "at the badge's left ({} vs {badge_left})",
        x_of(&still_a)
    );
    // No name will ever move: the pitch asks no repaint (the carry-over
    // frames drained above).
    window.pitch_paint_at(113.0, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_eq!(
        window.repaint_delay(),
        std::time::Duration::MAX,
        "Scroll names off: nothing is requested"
    );
}

// The draft's `nameSpans`: the eleven rows, the bench rows and the
// popover header draw the name with its colour codes.
#[test]
fn the_rows_and_the_popover_header_draw_the_names_colours() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    rows[5] = (
        "PEPSI MAN".to_owned(),
        "\u{11}cC9002BFFPEP\u{11}cFFFFFFFFSI \u{11}c004B93FFMAN".to_owned(),
        Some(5),
    );
    let roster = roster_rows(&rows);
    // Slot 5 is in the fixture's eleven: swap so it shows once. The swap
    // also makes the eleven non-identity, so a name read by the formation
    // index rather than the roster slot lands on marker and row 5, not 3.
    tactics.starting_eleven.swap(3, 5);
    assert_eq!(
        tactics.starting_eleven[3], 5,
        "premise: starter 3's roster slot is not his index"
    );
    tactics.bench_order[0] = 5;

    // The color of each painted "PEPSI MAN" galley's section covering
    // `run`'s first byte.
    let run_colors = |shapes: &[egui::epaint::ClippedShape], run: &str| {
        let start = "PEPSI MAN".find(run).expect("the run's offset");
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text == "PEPSI MAN" => Some(
                    shape
                        .galley
                        .job
                        .sections
                        .iter()
                        .find(|section| {
                            section.byte_range.start.0 <= start && start < section.byte_range.end.0
                        })
                        .unwrap_or_else(|| panic!("no section covering {run:?}"))
                        .format
                        .color,
                ),
                _ => None,
            })
            .collect::<Vec<egui::Color32>>()
    };

    let (nodes, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_eq!(
        lineup_rows(&nodes)[3].last().and_then(|name| name.value()),
        Some("PEPSI MAN"),
        "the eleven's row 3 names its roster slot's player"
    );
    let colors = run_colors(&shapes, "PEP");
    assert_eq!(colors.len(), 2, "the eleven row's and the bench row's name");
    for color in colors {
        assert_eq!(
            color,
            egui::Color32::from_rgb(0xc9, 0x00, 0x2b),
            "PEP's run keeps its code's colour"
        );
    }
    for color in run_colors(&shapes, "SI ") {
        assert_eq!(
            color,
            egui::Color32::from_rgb(0xff, 0xff, 0xff),
            "SI's run keeps its code's colour"
        );
    }
    for color in run_colors(&shapes, "MAN") {
        assert_eq!(
            color,
            egui::Color32::from_rgb(0x00, 0x4b, 0x93),
            "MAN's run keeps its code's colour"
        );
    }

    // The popover header's — and the marker's, which draws the same
    // coloured name below the badge (wider than the badge: two copies).
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let formation = tactics.presets[0].formations[0].clone();
    let at = marker_point(&nodes, &formation.players, 3);
    window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centre3 = centres_of(canvas, &formation.players)[3];
    let badge3 = egui::Rect::from_center_size(centre3, egui::vec2(BADGE_W, BADGE_H));
    // Marker 3's own label and the two copies of the name painted under
    // its badge (clipped to the badge's width, a name's height below it).
    let labeled: Vec<egui::Pos2> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.ends_with(" PEPSI MAN"))
        })
        .map(centre)
        .collect();
    assert_eq!(labeled.len(), 1, "one marker is labeled with the name");
    let marker3 = marker_rect(centre3, canvas).center();
    assert!(
        labeled[0].distance(marker3) <= 0.5,
        "the label is marker 3's ({:?}, marker 3 at {marker3:?})",
        labeled[0]
    );
    let under_badge = shapes
        .iter()
        .filter(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == "PEPSI MAN" => {
                let clip = clipped.clip_rect;
                (clip.left() - badge3.left()).abs() <= 0.5
                    && (clip.right() - badge3.right()).abs() <= 0.5
                    && (shape.pos.y - (centre3.y + NAME_DY - shape.galley.size().y / 2.0)).abs()
                        <= 0.5
            }
            _ => false,
        })
        .count();
    assert_eq!(
        under_badge, 2,
        "the name's two copies under marker 3's badge"
    );
    let colors = run_colors(&shapes, "PEP");
    assert_eq!(
        colors.len(),
        3,
        "the marker's two copies (\"PEPSI MAN\" is wide) and the popover header's name"
    );
    for color in colors {
        assert_eq!(
            color,
            egui::Color32::from_rgb(0xc9, 0x00, 0x2b),
            "PEP's run keeps its code's colour"
        );
    }
}

// The two lists share the span the host gives them (the draft's
// `#tac-lists`): each half of it less the 6-point gap, the eleven at its
// left, the bench ending at its right; the space `lineup` claims in the
// host runs from its top-left to that right edge and the lists' bottom.
// The host's cell starts away from the screen's origin, so a span or a
// height measured from the origin instead of the cell's top-left fails.
#[test]
fn the_lists_halve_the_span_and_the_lineup_claims_their_height() {
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let mut window = Headless::sized(800.0, 900.0);
    let cell = egui::Rect::from_min_max(egui::pos2(40.0, 30.0), egui::pos2(760.0, 880.0));
    let mut claimed = None;
    let (_, shapes) = window.frame(vec![], |ui| {
        let mut host = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(cell)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        super::lineup(
            &mut host,
            "lineup",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
        claimed = Some(host.min_rect());
    });
    let claimed = claimed.expect("the widget ran");
    // The lists' frames: the stroked rects taller than a row.
    let mut lists: Vec<egui::Rect> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if !shape.stroke.is_empty() && shape.rect.height() > 100.0 => {
                Some(shape.rect)
            }
            _ => None,
        })
        .collect();
    lists.sort_by(|a, b| a.min.x.total_cmp(&b.min.x));
    let [eleven, bench] = lists.as_slice() else {
        panic!("expected the two lists' frames, got {lists:?}")
    };
    let half = (cell.width() - 6.0) / 2.0;
    assert!(
        (eleven.min.x - cell.min.x).abs() <= 0.5,
        "the eleven at the span's left ({eleven:?})"
    );
    for (name, list) in [("eleven", eleven), ("bench", bench)] {
        assert!(
            (list.width() - half).abs() <= 0.5,
            "the {name} half of the span less the gap ({list:?}, half {half})"
        );
    }
    assert!(
        (bench.min.x - eleven.max.x - 6.0).abs() <= 0.5,
        "the lists 6 apart ({eleven:?}, {bench:?})"
    );
    assert!(
        (bench.max.x - cell.max.x).abs() <= 0.5,
        "the bench ends at the span's right ({bench:?})"
    );
    let lists_bottom = eleven.max.y.max(bench.max.y);
    assert_eq!(
        claimed.min, cell.min,
        "the claim starts at the cell's top-left"
    );
    assert!(
        (claimed.max.x - cell.max.x).abs() <= 0.5,
        "the claim spans to the cell's right ({claimed:?})"
    );
    assert!(
        (claimed.max.y - lists_bottom).abs() <= 0.5,
        "the claim ends at the lists' bottom ({claimed:?}, lists end at {lists_bottom})"
    );
}

// The lists keep the fixed width the host gives them beside the pitch: a
// name longer than the row ends cut at the list's edge, not drawn over the
// settings column at its right. "3DSMAX HAS STOPPED WORKING" is a /3/
// starter's name in the maintainer's PES 2021 save, which ran 50 points
// past the lists at the default window.
#[test]
fn a_long_name_is_cut_at_the_lists_width() {
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    let slot = usize::from(tactics.starting_eleven[2]);
    rows[slot].1 = "3DSMAX HAS STOPPED WORKING".to_owned();
    let roster = roster_rows(&rows);
    let mut window = Headless::sized(200.0, 900.0);
    let (nodes, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);

    let row = &lineup_rows(&nodes)[2];
    let name = row.last().expect("the row's name label");
    assert_eq!(
        name.value(),
        Some("3DSMAX HAS STOPPED WORKING"),
        "premise: the third starter's row names him"
    );
    // The starting eleven's list ends where its half of the width does,
    // 6 points before the bench's framed box (the right of the two
    // stroked rects taller than a row), which sits there whatever the
    // eleven's rows hold; a frame grown by a name could not bound it.
    let bench_left = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if !shape.stroke.is_empty() && shape.rect.height() > 100.0 => {
                Some(shape.rect.min.x)
            }
            _ => None,
        })
        .max_by(f32::total_cmp)
        .expect("the bench's frame");
    let list_right = bench_left - 6.0;
    assert!(
        list_right < 150.0,
        "premise: the eleven's list ends well inside the window ({list_right})"
    );
    let bounds = name.bounds().expect("name bounds");
    assert!(
        bounds.x1 <= f64::from(list_right),
        "the name ends inside the eleven's list ({bounds:?}, list ends at {list_right})"
    );
}

// The swap mark. A pitch player dropped on another swaps the two slots
// in the shown formation, so while a dragged starter is held over
// another, that one carries the selected player's corner brackets drawn
// over everything and the dragged marker goes to 35% opacity: a drag
// meant only to move shows the swap before the release and can be
// steered off it. Over another marker the brackets show, painted after
// every shape of the dragged marker, and his badge fades; over the
// grass, neither.
#[test]
fn a_drag_over_another_marker_marks_the_swap_and_dims_the_dragged() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    let players = tactics.presets[0].formations[0].players;
    let (dragged, target) = (3, 6);
    // A portrait for the dragged starter, so his marker paints one the
    // brackets must cover.
    let texture = window.ctx.load_texture(
        "a portrait",
        egui::ColorImage::new([4, 4], vec![egui::Color32::RED; 16]),
        egui::TextureOptions::LINEAR,
    );
    roster[usize::from(tactics.starting_eleven[dragged])].portrait = Some(texture.id());

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let from = marker_point(&nodes, &players, dragged);
    // `over` is on the target's marker and off the dragged one's own
    // spot; `target` is the topmost other marker there. It is two units
    // right of the target's own free point: the held marker stands at
    // the pointer less the press's offset, and from the target's point
    // he would sit exactly on the target's badge, where the two badges'
    // rects cannot be told apart.
    let over = marker_point(&nodes, &players, target) + egui::vec2(2.0 * SPOT_XS, 0.0);
    assert!(
        !marker_rect(centres[dragged], canvas).contains(over),
        "premise: the hold is off the dragged marker's own spot"
    );
    assert_eq!(
        swap_target(&players, &centres, canvas, over, dragged),
        Some(target),
        "premise: the hold is on the target's marker, under no later one"
    );

    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(from, true),
    );
    let (_, shapes) = window.frame(vec![egui::Event::PointerMoved(over)], |ui| {
        pitch(
            ui,
            "pitch",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    // The target's marker carries the selected corners: eight 9 px
    // arms, each ending on a corner of its portrait box.
    let badge = egui::Rect::from_center_size(centres[target], egui::vec2(50.0, 14.0));
    let portrait = egui::Rect::from_min_size(
        egui::pos2(badge.left(), badge.top() - 50.0),
        egui::vec2(50.0, 50.0),
    );
    let arms: Vec<[egui::Pos2; 2]> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == Palette::DARK.selected_corners =>
            {
                Some(*points)
            }
            _ => None,
        })
        .collect();
    assert_eq!(arms.len(), 8, "the swap mark's eight corner arms");
    let box_corners = [
        portrait.left_top(),
        portrait.right_top(),
        portrait.left_bottom(),
        portrait.right_bottom(),
    ];
    for points in &arms {
        assert!(
            (points[0].distance(points[1]) - 9.0).abs() <= 0.01,
            "a 9 px arm"
        );
        assert!(
            box_corners.iter().any(|corner| points.contains(corner)),
            "{points:?} ends on the target's portrait box"
        );
    }
    // And the dragged marker's badge paints at 35% over him.
    let dragged_badge = egui::Rect::from_center_size(
        held_centre(canvas, players[dragged], from, over),
        egui::vec2(50.0, 14.0),
    );
    let fill = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.rect == dragged_badge => Some(shape.fill),
            _ => None,
        })
        .next()
        .expect("the dragged marker's badge");
    assert_eq!(
        fill,
        Palette::DARK.badge_fill.gamma_multiply(0.35),
        "the dragged badge at 35% opacity"
    );
    // The brackets are drawn over everything: every arm after the
    // dragged marker's portrait and badge.
    let dragged_shapes: Vec<usize> = shapes
        .iter()
        .enumerate()
        .filter(|(_, clipped)| match &clipped.shape {
            egui::Shape::Rect(shape) => shape.rect == dragged_badge,
            shape @ egui::Shape::Mesh(_) => shape.texture_id() == texture.id(),
            _ => false,
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        dragged_shapes.len(),
        2,
        "premise: the dragged marker's portrait and badge"
    );
    let arm_indices: Vec<usize> = shapes
        .iter()
        .enumerate()
        .filter(|(_, clipped)| {
            matches!(
                &clipped.shape,
                egui::Shape::LineSegment { stroke, .. } if stroke.color == Palette::DARK.selected_corners
            )
        })
        .map(|(index, _)| index)
        .collect();
    for arm in &arm_indices {
        assert!(
            dragged_shapes.iter().all(|shape| shape < arm),
            "arm shape {arm} painted after the dragged marker's shapes {dragged_shapes:?}"
        );
    }

    // Held over the grass instead: no brackets, full opacity.
    let grass = grass_point(canvas, &players, egui::Rect::NOTHING);
    let (_, shapes) = window.frame(vec![egui::Event::PointerMoved(grass)], |ui| {
        pitch(
            ui,
            "pitch",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    assert!(
        !shapes.iter().any(|clipped| matches!(
            &clipped.shape,
            egui::Shape::LineSegment { stroke, .. } if stroke.color == Palette::DARK.selected_corners
        )),
        "no brackets over the grass"
    );
    let dragged_badge = egui::Rect::from_center_size(
        held_centre(canvas, players[dragged], from, grass),
        egui::vec2(50.0, 14.0),
    );
    let fill = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.rect == dragged_badge => Some(shape.fill),
            _ => None,
        })
        .next()
        .expect("the dragged marker's badge");
    assert_eq!(
        fill,
        Palette::DARK.badge_fill,
        "full opacity over the grass"
    );
}

// A starter's badge carries his medal tier as a 1.5-point outline
// inside the badge's edge, in the tier's colour — a gold `#cc9900`,
// a silver `#c0c0c0`, a bronze `#8c4a20`; a slot with no medal draws
// none, and no letter is painted.
#[test]
fn a_medal_outlines_the_badge_in_its_tiers_colour() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    // A non-identity eleven: a medal keyed by the marker's index rather
    // than its roster slot would look right on the fixture's [0, 1, ...].
    tactics.starting_eleven.swap(4, 9);
    assert_ne!(
        tactics.starting_eleven[4], 4,
        "premise: slot and index differ"
    );
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    let medals = [
        (
            4usize,
            Medal::Gold,
            egui::Color32::from_rgb(0xcc, 0x99, 0x00),
        ),
        (5, Medal::Silver, egui::Color32::from_rgb(0xc0, 0xc0, 0xc0)),
        (6, Medal::Bronze, egui::Color32::from_rgb(0x8c, 0x4a, 0x20)),
    ];
    for (index, medal, _) in medals {
        roster[usize::from(tactics.starting_eleven[index])].medal = Some(medal);
    }
    let players = tactics.presets[0].formations[0].players;

    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    /// The stroked rect covering `badge` (within 0.5), `None` when
    /// none was painted.
    fn outline_at(
        shapes: &[egui::epaint::ClippedShape],
        badge: egui::Rect,
    ) -> Option<&egui::epaint::RectShape> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if !shape.stroke.is_empty()
                        && (shape.rect.left() - badge.left()).abs() <= 0.5
                        && (shape.rect.right() - badge.right()).abs() <= 0.5
                        && (shape.rect.top() - badge.top()).abs() <= 0.5
                        && (shape.rect.bottom() - badge.bottom()).abs() <= 0.5 =>
                {
                    Some(shape)
                }
                _ => None,
            })
            .next()
    }
    for (index, _) in players.iter().enumerate() {
        let badge = egui::Rect::from_center_size(centres[index], egui::vec2(50.0, 14.0));
        match medals.iter().find(|(slot, _, _)| *slot == index) {
            Some((_, _, colour)) => {
                let outline = outline_at(&shapes, badge)
                    .unwrap_or_else(|| panic!("no outline over marker {index}'s badge"));
                assert_eq!(outline.stroke.width, 1.5, "the outline's width");
                assert_eq!(
                    outline.stroke_kind,
                    egui::StrokeKind::Inside,
                    "the outline inside the badge's edge"
                );
                assert_eq!(
                    outline.stroke.color, *colour,
                    "marker {index}'s outline in its tier's colour"
                );
            }
            None => assert!(
                outline_at(&shapes, badge).is_none(),
                "marker {index} has no medal: no outline"
            ),
        }
    }
    // The tiers paint no letter.
    assert_eq!(
        shapes
            .iter()
            .filter(|clipped| matches!(
                &clipped.shape,
                egui::Shape::Text(shape)
                    if matches!(shape.galley.job.text.as_str(), "G" | "S" | "B")
                        && canvas.contains(shape.visual_bounding_rect().center())
            ))
            .count(),
        0,
        "no medal letter is painted"
    );
}

// The badge's right end shows the starter's overall at his slot's
// position: the row's `overalls` entry the position byte indexes,
// painted white at 11.5 with its galley's right edge 2 in. A row
// holding none paints no number, and a position change paints the new
// position's entry on the next frame.
#[test]
fn the_badge_paints_the_overall_at_the_slots_position() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    // A non-identity eleven: an overall keyed by the marker's index
    // rather than its roster slot would look right on the fixture's
    // [0, 1, ...].
    tactics.starting_eleven.swap(4, 9);
    assert_ne!(
        tactics.starting_eleven[4], 4,
        "premise: slot and index differ"
    );
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    // Each starter's thirteen, an entry per position distinct per slot
    // (synthetic numbers: they are a widget input, not a record); the
    // starter at index 3's row holds none.
    for (index, &roster_slot) in tactics.starting_eleven.iter().enumerate() {
        if index == 3 {
            continue;
        }
        let base = 40 + roster_slot;
        let mut overalls = [0; 13];
        for (position, overall) in overalls.iter_mut().enumerate() {
            *overall = base + u8::try_from(position).expect("thirteen positions");
        }
        roster[usize::from(roster_slot)].overalls = Some(overalls);
    }
    let players = tactics.presets[0].formations[0].players;
    // The text shapes painted at `badge`'s right end: a galley's right
    // edge 2 in, centred on the badge.
    fn number_at(
        shapes: &[egui::epaint::ClippedShape],
        badge: egui::Rect,
    ) -> Vec<&egui::epaint::TextShape> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape)
                    if (shape.pos.x + shape.galley.size().x - (badge.right() - 2.0)).abs()
                        <= 0.5
                        && (shape.pos.y + shape.galley.size().y / 2.0 - badge.center().y).abs()
                            <= 0.5 =>
                {
                    Some(shape)
                }
                _ => None,
            })
            .collect()
    }

    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    for (index, slot) in players.iter().enumerate() {
        let badge = egui::Rect::from_center_size(centres[index], egui::vec2(50.0, 14.0));
        let found = number_at(&shapes, badge);
        match roster[usize::from(tactics.starting_eleven[index])].overalls {
            Some(overalls) => {
                let [number] = found.as_slice() else {
                    panic!("one number at marker {index}'s badge, got {}", found.len())
                };
                assert_eq!(
                    number.galley.job.text,
                    overalls[usize::from(slot.position)].to_string(),
                    "marker {index}'s overall at his slot's position"
                );
                assert_eq!(
                    painted_color(number),
                    egui::Color32::WHITE,
                    "marker {index}'s overall white as the game's"
                );
                assert_eq!(
                    number.galley.job.sections[0].format.font_id,
                    egui::FontId::proportional(11.5),
                    "marker {index}'s overall at the badge texts' size"
                );
            }
            None => assert!(
                found.is_empty(),
                "marker {index} holds no overalls: no number"
            ),
        }
    }

    // A position change paints the new position's entry on the next
    // frame.
    let index = 4;
    let position = players[index].position;
    let new_position = (1..13)
        .find(|p| *p != position)
        .expect("another outfield position");
    tactics.presets[0].formations[0].players[index].position = new_position;
    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let players = tactics.presets[0].formations[0].players;
    let centres = centres_of(canvas, &players);
    let badge = egui::Rect::from_center_size(centres[index], egui::vec2(50.0, 14.0));
    let found = number_at(&shapes, badge);
    let [number] = found.as_slice() else {
        panic!("the moved starter's number at the new position")
    };
    let overalls = roster[usize::from(tactics.starting_eleven[index])]
        .overalls
        .expect("starter 4's overalls");
    assert_eq!(
        number.galley.job.text,
        overalls[usize::from(new_position)].to_string(),
        "his number follows the slot's new position"
    );
}

/// A `FieldedMark` on `preset` (0-based) at `position` byte, its severity
/// and tooltip line.
fn fielded(preset: usize, position: u8, error: bool, line: &str) -> FieldedMark {
    FieldedMark {
        preset,
        position,
        error,
        line: line.to_owned(),
    }
}

/// The frame's line segments 5 or more wide, in paint order, with their
/// index in `shapes`: the pitch crosses' strokes (the pitch's lines and
/// corners are 2 wide), four a cross, its two black outlines then its two
/// coloured strokes.
fn cross_strokes(
    shapes: &[egui::epaint::ClippedShape],
) -> Vec<(usize, [egui::Pos2; 2], egui::Stroke)> {
    shapes
        .iter()
        .enumerate()
        .filter_map(|(index, clipped)| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke } if stroke.width >= 5.0 => {
                Some((index, *points, *stroke))
            }
            _ => None,
        })
        .collect()
}

/// The frame's pitch crosses: their coloured strokes' colour, one entry
/// a cross.
fn crosses(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::Color32> {
    cross_strokes(shapes)
        .chunks(4)
        .map(|cross| cross[3].2.color)
        .collect()
}

/// The portrait box above the badge centred at `centre`: 50 square, its
/// bottom on the badge's top.
fn portrait_at(centre: egui::Pos2) -> egui::Rect {
    let badge = egui::Rect::from_center_size(centre, egui::vec2(50.0, 14.0));
    egui::Rect::from_min_max(
        egui::pos2(badge.left(), badge.top() - 50.0),
        egui::pos2(badge.right(), badge.top()),
    )
}

// Live AATF's `FieldedNotA` cross: a starter whose row's `fielded` marks
// his spot on the shown formation carries an X over his whole portrait,
// corner to corner 6 in, its two 5-wide strokes in the error colour when
// one mark there is an Error over the same strokes in black 7 wide. It is
// painted after every marker, so a marker drawn later cannot cover it,
// and no `✖` glyph is drawn.
#[test]
fn a_fielded_mark_paints_an_x_over_the_starters_portrait() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    assert_eq!(
        tactics.presets[0].style.fluid,
        Some(false),
        "premise: the shown preset is not fluid"
    );
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    let index = 4;
    // A non-identity eleven: marks looked up by the marker's index rather
    // than its roster slot would look right on the fixture's [0, 1, ...].
    tactics.starting_eleven.swap(index, 9);
    assert_ne!(
        usize::from(tactics.starting_eleven[index]),
        index,
        "premise: the starter's slot and index differ"
    );
    let slot = usize::from(tactics.starting_eleven[index]);
    let position = tactics.presets[0].formations[0].players[index].position;
    let marks = [fielded(
        0,
        position,
        true,
        "FieldedNotA: fielded at AMF in preset 1 without an A rating",
    )];
    roster[slot].fielded = &marks;
    let players = tactics.presets[0].formations[0].players;

    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let portrait = portrait_at(centres_of(canvas, &players)[index]);
    let strokes = cross_strokes(&shapes);
    let [black_a, black_b, red_a, red_b] = strokes.as_slice() else {
        panic!("expected one cross's four strokes, got {}", strokes.len())
    };
    let error = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .error_fg_color;
    // The two diagonals, corner to corner of the portrait 6 in.
    let inner = portrait.shrink(6.0);
    let diagonals = [
        [inner.left_top(), inner.right_bottom()],
        [inner.right_top(), inner.left_bottom()],
    ];
    for ((_, points, stroke), (color, width)) in [black_a, black_b, red_a, red_b].into_iter().zip([
        (egui::Color32::BLACK, 7.0),
        (egui::Color32::BLACK, 7.0),
        (error, 5.0),
        (error, 5.0),
    ]) {
        assert_eq!(
            (stroke.color, stroke.width),
            (color, width),
            "black under, the coloured strokes over"
        );
        assert!(
            diagonals.iter().any(|diagonal| diagonal
                .iter()
                .zip(points)
                .all(|(want, got)| (*want - *got).length() <= 0.01)),
            "a stroke from {points:?} is no diagonal of the portrait {portrait:?} 6 in"
        );
    }
    assert!(
        !shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(shape) if shape.galley.job.text == CROSS.to_string())
        }),
        "no `✖` glyph"
    );

    // The cross's shapes come after every marker's shapes in the
    // frame's list (every marker paints a badge, so after the last
    // badge is after every marker).
    let cross_index = black_a.0;
    let last_badge = shapes
        .iter()
        .rposition(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(shape) if shape.fill == Palette::DARK.badge_fill)
        })
        .expect("a badge shape");
    assert!(
        cross_index > last_badge,
        "the cross paints after every marker"
    );
}

// The cross draws only while the shown formation is one the shown
// preset fields and the mark names the slot's position there: a
// non-fluid preset fields its kick-off formation alone (its other two
// carry none, as does another preset's whole pitch); a fluid preset
// fields all three, so a mark at the position only formation 3 puts
// him at crosses on formation 3 alone.
#[test]
fn the_cross_draws_only_on_a_formation_the_preset_fields() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    assert_eq!(
        tactics.presets[0].style.fluid,
        Some(false),
        "premise: preset 1 is not fluid"
    );
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    let index = 4;
    let slot = usize::from(tactics.starting_eleven[index]);
    let position = tactics.presets[0].formations[0].players[index].position;
    let marks = [fielded(
        0,
        position,
        true,
        "FieldedNotA: fielded at AMF in preset 1 without an A rating",
    )];
    roster[slot].fielded = &marks;

    // The kick-off formation carries it; the same preset's second
    // formation is never shown, the pitch going back to Kick-off and its
    // cross; another preset's pitch carries none.
    let (_, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_eq!(crosses(&shapes).len(), 1, "the kick-off formation's cross");
    view.formation = 1;
    let (_, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_eq!(view.formation, 0, "a non-fluid preset shows Kick-off alone");
    assert_eq!(crosses(&shapes).len(), 1, "and Kick-off's cross");
    view.preset = 1;
    view.formation = 0;
    let (_, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert!(
        crosses(&shapes).is_empty(),
        "another preset's pitch carries none"
    );

    // A fluid preset fields all three: the mark at the position only
    // formation 3 puts him at crosses there alone.
    view.preset = 0;
    tactics.presets[0].style.fluid = Some(true);
    let pos0 = tactics.presets[0].formations[0].players[index].position;
    let pos1 = tactics.presets[0].formations[1].players[index].position;
    let pos2 = (0..13)
        .find(|p| *p != pos0 && *p != pos1)
        .expect("a position not his in the other two");
    tactics.presets[0].formations[2].players[index].position = pos2;
    let marks = [fielded(
        0,
        pos2,
        true,
        "FieldedNotA: fielded at GK in preset 1 without an A rating",
    )];
    roster[slot].fielded = &marks;
    for (formation, want) in [(0usize, 0usize), (1, 0), (2, 1)] {
        view.formation = formation;
        let (_, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
        assert_eq!(
            crosses(&shapes).len(),
            want,
            "formation {formation}'s crosses"
        );
    }
}

// A mark that is only a Warning paints the pitch's warning colour (the
// severity's colour is the highest among the spot's marks): the dark
// theme's orange on both themes, since the turf under it is the same in
// both, where the light theme's own warning colour is darkened for its panel.
#[test]
fn a_warning_only_mark_paints_the_pitchs_warn_colour_on_both_themes() {
    let light_warn = Headless::sized(600.0, 450.0)
        .ctx
        .style_of(egui::Theme::Light)
        .visuals
        .warn_fg_color;
    assert_ne!(
        Palette::LIGHT.pitch_warn,
        light_warn,
        "premise: the light theme's own warning colour is not the pitch's"
    );
    for (theme, palette) in [
        (egui::Theme::Dark, &Palette::DARK),
        (egui::Theme::Light, &Palette::LIGHT),
    ] {
        let mut window = Headless::sized(600.0, 450.0);
        window.ctx.set_theme(theme);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let mut roster = roster_rows(&rows);
        let index = 4;
        let slot = usize::from(tactics.starting_eleven[index]);
        let position = tactics.presets[0].formations[0].players[index].position;
        let marks = [fielded(
            0,
            position,
            false,
            "FieldedNotA: fielded at AMF in preset 1 without an A rating",
        )];
        roster[slot].fielded = &marks;

        let (_, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
        let found = crosses(&shapes);
        let [color] = found.as_slice() else {
            panic!(
                "expected one cross on the {theme:?} theme, got {}",
                found.len()
            )
        };
        assert_eq!(
            *color, palette.pitch_warn,
            "the pitch's warning colour on the {theme:?} theme"
        );
        assert_eq!(
            *color,
            egui::Color32::from_rgb(0xFF, 0x8F, 0x00),
            "the dark theme's orange on the {theme:?} theme"
        );
    }
}

// The hover over the crossed portrait shows the marks' `line`s, one per
// line, as a field mark's tooltip shows the violations' lines; a press on
// it still takes the marker (the tooltip senses hover alone).
#[test]
fn the_crosss_hover_shows_the_marks_lines() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    let index = 4;
    let slot = usize::from(tactics.starting_eleven[index]);
    let position = tactics.presets[0].formations[0].players[index].position;
    let line = "FieldedNotA: fielded at AMF in preset 1 without an A rating";
    let marks = [
        fielded(0, position, true, line),
        fielded(0, position, false, line),
    ];
    roster[slot].fielded = &marks;

    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert_eq!(crosses(&shapes).len(), 1, "premise: the cross is drawn");
    let players = tactics.presets[0].formations[0].players;
    // Off the strokes' crossing, still on the portrait.
    let on_portrait = portrait_at(centres_of(canvas_rect(&nodes), &players)[index]).center()
        + egui::vec2(0.0, 12.0);
    let tip = window.pitch_tooltip(
        on_portrait,
        "FieldedNotA",
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert_eq!(
        tip.as_deref(),
        Some(format!("{line}\n{line}").as_str()),
        "the marks' lines, one per line"
    );
    assert_eq!(view.popover, None, "premise: no popover open");
    window.click_pitch(
        on_portrait,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(index),
        "a press on the crossed portrait takes his marker"
    );
}

/// A formation of eleven `(position, x, y)` slots, the position by its
/// label.
fn formation_of(slots: [(&str, u8, u8); 11]) -> Formation {
    Formation {
        players: slots.map(|(label, x, y)| FormationSlot {
            position: POSITIONS
                .iter()
                .position(|name| *name == label)
                .and_then(|index| u8::try_from(index).ok())
                .unwrap_or_else(|| panic!("{label} is a position")),
            x,
            y,
        }),
    }
}

/// The eight formations PES 2021's formation screen was read on
/// (2026-10-27), as the draft's PES 21 sample save holds them, with the
/// name the screen wrote: /3/'s (701) Kick-off, /f/'s (718) preset 1's
/// three, /gd/'s (722) presets 1 to 3, /o/'s (739).
fn captured_formations() -> [(&'static str, Formation, &'static str); 8] {
    [
        (
            "/3/ Kick-off",
            formation_of([
                ("GK", 52, 3),
                ("CB", 51, 8),
                ("CB", 29, 8),
                ("CB", 74, 8),
                ("DMF", 53, 17),
                ("CMF", 77, 24),
                ("CMF", 27, 24),
                ("SS", 69, 44),
                ("SS", 50, 36),
                ("AMF", 52, 27),
                ("CF", 37, 45),
            ]),
            "3-3-1-3",
        ),
        (
            "/f/ preset 1 Kick-off",
            formation_of([
                ("GK", 52, 3),
                ("SS", 69, 35),
                ("CMF", 53, 26),
                ("DMF", 66, 17),
                ("CB", 32, 6),
                ("CB", 72, 6),
                ("CB", 52, 7),
                ("RMF", 84, 27),
                ("LMF", 20, 26),
                ("DMF", 42, 17),
                ("CF", 42, 41),
            ]),
            "3-3-2-2",
        ),
        (
            "/f/ preset 1 In possession",
            formation_of([
                ("GK", 52, 3),
                ("SS", 65, 38),
                ("CMF", 54, 26),
                ("DMF", 76, 25),
                ("CB", 32, 6),
                ("CB", 72, 6),
                ("CB", 52, 7),
                ("RWF", 87, 35),
                ("LWF", 18, 35),
                ("DMF", 32, 25),
                ("CF", 44, 40),
            ]),
            "3-3-4",
        ),
        (
            "/f/ preset 1 Out of possession",
            formation_of([
                ("GK", 52, 3),
                ("CMF", 63, 26),
                ("CMF", 43, 26),
                ("DMF", 65, 16),
                ("CB", 32, 6),
                ("CB", 72, 6),
                ("CB", 52, 7),
                ("RMF", 84, 27),
                ("LMF", 20, 26),
                ("DMF", 42, 16),
                ("CF", 43, 40),
            ]),
            "3-4-2-1",
        ),
        (
            "/gd/ preset 1",
            formation_of([
                ("GK", 52, 3),
                ("CB", 64, 11),
                ("CB", 40, 11),
                ("RB", 88, 13),
                ("LB", 16, 13),
                ("CMF", 69, 24),
                ("SS", 70, 42),
                ("DMF", 41, 21),
                ("AMF", 53, 33),
                ("SS", 35, 42),
                ("CF", 53, 46),
            ]),
            "4-2-1-3",
        ),
        (
            "/gd/ preset 2",
            formation_of([
                ("GK", 52, 3),
                ("CB", 64, 11),
                ("CB", 40, 11),
                ("RB", 88, 13),
                ("LB", 16, 13),
                ("CMF", 32, 25),
                ("CMF", 73, 25),
                ("DMF", 52, 19),
                ("AMF", 52, 32),
                ("SS", 40, 43),
                ("CF", 65, 45),
            ]),
            "4-3-1-2",
        ),
        (
            "/gd/ preset 3",
            formation_of([
                ("GK", 52, 3),
                ("CB", 64, 11),
                ("CB", 40, 11),
                ("RB", 88, 13),
                ("LB", 16, 13),
                ("CMF", 28, 25),
                ("CMF", 77, 25),
                ("DMF", 52, 19),
                ("AMF", 65, 34),
                ("AMF", 42, 34),
                ("CF", 53, 45),
            ]),
            "4-3-2-1",
        ),
        (
            "/o/ preset 1",
            formation_of([
                ("GK", 52, 3),
                ("CB", 52, 9),
                ("CB", 72, 9),
                ("CMF", 71, 19),
                ("CB", 32, 9),
                ("AMF", 51, 33),
                ("AMF", 73, 31),
                ("CMF", 33, 19),
                ("AMF", 30, 31),
                ("CMF", 52, 21),
                ("CF", 52, 43),
            ]),
            "3-3-3-1",
        ),
    ]
}

// The formation's name: the eight formations read off PES 2021's
// formation screen get the name the screen wrote (/f/'s In
// possession, no wide or attacking midfielder, leaves its empty row out:
// `3-3-4`).
#[test]
fn the_formation_name_counts_the_positions_grids_rows_back_to_front() {
    for (what, formation, name) in captured_formations() {
        assert_eq!(formation_name(&formation), name, "{what}");
    }
}

// The position counts, not the spot: /3/'s AMF given CMF at his own spot
// reads `3-4-3` (the screen's capture c3_11); the keeper never counts,
// wherever he stands.
#[test]
fn the_formation_name_follows_the_position_and_never_counts_the_keeper() {
    let (_, mut formation, name) = captured_formations()[0].clone();
    assert_eq!(name, "3-3-1-3", "premise: /3/'s Kick-off");
    let amf = formation
        .players
        .iter()
        .position(|slot| position_label(slot.position) == "AMF")
        .expect("premise: /3/ fields an AMF");
    formation.players[amf].position = 5;
    assert_eq!(position_label(5), "CMF", "premise: 5 is CMF");
    assert_eq!(formation_name(&formation), "3-4-3");
    let keeper = formation
        .players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("premise: a keeper slot");
    formation.players[keeper].x = 52;
    formation.players[keeper].y = 40;
    assert_eq!(
        formation_name(&formation),
        "3-4-3",
        "the keeper's spot plays no part"
    );
}

// The shown formation's name sits in the canvas' bottom-left corner,
// white, in the `label` family at 20.5 pt, its text origin 6.5 in from
// the canvas' left edge and its baseline 6 above the bottom edge (the
// draft's `fillText` at `PITCH.h - 6`, alphabetic baseline), painted
// after every marker so a low one never covers it.
#[test]
fn the_formation_name_sits_in_the_bottom_left_corner_after_the_markers() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let (_, formation, name) = captured_formations()[0].clone();
    tactics.presets[0].formations[0] = formation;

    let (nodes, shapes) = window.pitch_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let canvas = canvas_rect(&nodes);
    let (index, shape) = shapes
        .iter()
        .enumerate()
        .find_map(|(index, clipped)| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == name => Some((index, shape)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no painted {name:?}"));
    let row = &shape.galley.rows[0];
    let glyph = row.glyphs.first().expect("the name's first glyph");
    let origin = shape.pos + row.pos.to_vec2() + glyph.pos.to_vec2();
    assert!(
        (origin.x - (canvas.left() + 6.5)).abs() <= 0.01,
        "the text origin 6.5 in from the left edge ({origin:?}, canvas {canvas:?})"
    );
    assert!(
        (origin.y - (canvas.bottom() - 6.0)).abs() <= 0.01,
        "the baseline 6 above the bottom edge ({origin:?}, canvas {canvas:?})"
    );
    assert_eq!(painted_color(shape), egui::Color32::WHITE, "white");
    assert_eq!(
        shape.galley.job.sections[0].format.font_id,
        egui::FontId::new(20.5, egui::FontFamily::Name("label".into())),
        "the label family at 20.5 pt"
    );
    let last_badge = shapes
        .iter()
        .rposition(
            |clipped| matches!(&clipped.shape, egui::Shape::Rect(rect) if rect.fill == Palette::DARK.badge_fill),
        )
        .expect("the markers' badges");
    assert!(
        index > last_badge,
        "the name paints after every marker ({index} vs badge {last_badge})"
    );
}

/// The marker boxes the frame's buttons cover: a marker's AccessKit
/// bounds are his box.
fn button_boxes(nodes: &[accesskit::Node]) -> Vec<egui::Rect> {
    nodes
        .iter()
        .filter(|node| node.role() == Role::Button)
        .filter_map(|node| node.bounds())
        .map(|b| {
            egui::Rect::from_min_max(
                egui::pos2(b.x0 as f32, b.y0 as f32),
                egui::pos2(b.x1 as f32, b.y1 as f32),
            )
        })
        .collect()
}

// The formation selector follows the formations the shown preset fields.
// On the PES 21 save's team 718, preset 1 with Fluid formation on, its
// three tabs are enabled. With Fluid formation switched off in the
// record, the next frame draws them disabled with Kick-off selected, the
// view back on Kick-off although it showed Out-of-possession, a click on
// In-possession leaves it there, and the pitch draws Kick-off's slots.
#[test]
fn the_formation_selector_is_locked_on_kick_off_while_fluid_formation_is_off() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics =
        pes_savefile::EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21))
            .expect("fixture save")
            .team(718)
            .expect("team 718 is in the fixture")
            .tactics
            .clone();
    tactics.presets[0].style.fluid = Some(true);
    // Out-of-possession's slot 5 a few units off Kick-off's, so the two
    // formations' markers can be told apart.
    tactics.presets[0].formations[2].players[5].x += 6;
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let tabs = |nodes: &[accesskit::Node]| {
        FORMATIONS.map(|label| entry(nodes, Role::Button, label).is_disabled())
    };

    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(tabs(&nodes), [false; 3], "Fluid formation on: three tabs");

    tactics.presets[0].style.fluid = Some(false);
    view.formation = 2;
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(tabs(&nodes), [true; 3], "Fluid formation off: locked");
    assert_eq!(view.formation, 0, "the view is back on Kick-off");
    assert_eq!(
        FORMATIONS.map(|label| entry(&nodes, Role::Button, label).toggled()),
        [
            Some(accesskit::Toggled::True),
            Some(accesskit::Toggled::False),
            Some(accesskit::Toggled::False),
        ],
        "Kick-off is selected"
    );
    let canvas = canvas_rect(&nodes);
    let boxes = button_boxes(&nodes);
    let shown = |formation: usize| {
        let slot = tactics.presets[0].formations[formation].players[5];
        let marker = marker_rect(badge_centre(canvas.min, slot), canvas);
        boxes.iter().any(|b| (b.min - marker.min).length() <= 0.5)
    };
    assert!(shown(0), "Kick-off's slot 5 is on the pitch");
    assert!(!shown(2), "Out-of-possession's is not");

    let at = centre(entry(&nodes, Role::Button, "In-possession"));
    window.pitch_and_lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(at, true),
    );
    window.pitch_and_lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(at, false),
    );
    assert_eq!(
        view.formation, 0,
        "a click on In-possession changes nothing"
    );
}

// PES 2015 has no Fluid formation: its selector is always locked.
#[test]
fn the_formation_selector_is_locked_on_pes_15() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes15);
    assert_eq!(
        tactics.presets[0].style.fluid, None,
        "premise: PES 2015 stores no Fluid formation"
    );
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes15);
    let roster = roster_rows(&rows);
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes15, &mut tactics, &mut view, &roster, vec![]);
    for label in FORMATIONS {
        assert!(
            entry(&nodes, Role::Button, label).is_disabled(),
            "{label} is disabled"
        );
    }
}
