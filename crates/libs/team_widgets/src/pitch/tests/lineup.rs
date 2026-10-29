//! The lineup lists' rows: the drag handle's insets and the positions in
//! the pitch badges' colours.

use pes_version::PesVersion;
use studio_core::widgets::ink_span;

use super::*;
use crate::test_support::{Headless, fixture_rows, fixture_tactics, painted_color, roster_rows};

/// The text shape painted for the Label node `node`: its own text, laid
/// out where the node stands.
fn shape_of<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    node: &accesskit::Node,
) -> &'a egui::epaint::TextShape {
    let bounds = node.bounds().expect("a label's bounds");
    let text = node.value().expect("a label's text");
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if shape.galley.job.text == text
                    && (f64::from(shape.pos.y) - bounds.y0).abs() < 3.0
                    && f64::from(shape.pos.x) >= bounds.x0 - 1.0
                    && f64::from(shape.pos.x) <= bounds.x1 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted {text:?} at {bounds:?}"))
}

/// Where `shape`'s glyphs paint, on the screen's x axis.
fn ink_x(shape: &egui::epaint::TextShape) -> egui::Rangef {
    let span = ink_span(&shape.galley);
    egui::Rangef::new(shape.pos.x + span.min, shape.pos.x + span.max)
}

// A row's drag handle stands 4 points from the row's left edge and 4
// before the position, in ink, so the name gets the room; the rows run
// from the list's frame (the draft's `.lrow` inside `.llist`), so the
// handle's inset is the row's own.
#[test]
fn the_handle_stands_4_from_the_rows_edge_and_4_before_the_position() {
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let mut window = Headless::sized(800.0, 900.0);
    let (nodes, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let frames: Vec<egui::epaint::RectShape> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if !shape.stroke.is_empty() && shape.rect.height() > 100.0 => {
                Some(shape.clone())
            }
            _ => None,
        })
        .collect();
    let lineup = lineup_rows(&nodes);
    // The eleven's first row and the bench's first.
    for row in [&lineup[0], &lineup[11]] {
        let [handle, position, _name] = row.as_slice() else {
            panic!("a row's three labels, got {row:?}")
        };
        let handle_bounds = handle.bounds().expect("the handle's bounds");
        let frame = frames
            .iter()
            .find(|frame| {
                f64::from(frame.rect.min.x) < handle_bounds.x0
                    && f64::from(frame.rect.max.x) > handle_bounds.x1
            })
            .expect("the handle's list frame");
        let edge = frame.rect.min.x + frame.stroke.width;
        let row_left = nodes
            .iter()
            .filter(|node| node.role() == Role::Unknown)
            .filter_map(accesskit::Node::bounds)
            .find(|bounds| {
                bounds.x0 <= handle_bounds.x0
                    && bounds.y0 <= handle_bounds.y0
                    && bounds.y1 >= handle_bounds.y1
                    && bounds.y1 - bounds.y0 < 30.0
            })
            .expect("the row's drag source")
            .x0;
        assert!(
            (row_left - f64::from(edge)).abs() <= 0.5,
            "the row starts at the list's frame ({row_left} vs {edge})"
        );
        let handle_ink = ink_x(shape_of(&shapes, handle));
        let position_ink = ink_x(shape_of(&shapes, position));
        let inset = f64::from(handle_ink.min) - row_left;
        assert!(
            (inset - 4.0).abs() <= 0.5,
            "the handle's ink {inset} from the row's edge"
        );
        let gap = position_ink.min - handle_ink.max;
        assert!(
            (gap - 4.0).abs() <= 0.5,
            "the handle's ink {gap} before the position's"
        );
    }
}

// Each row's position is drawn in its group's colour, the pitch badge's
// (`position_colour`): the formation's position for a starter, the
// registered one for a bench player; the names keep their own colours.
#[test]
fn each_rows_position_is_in_its_groups_badge_colour() {
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let mut window = Headless::sized(800.0, 900.0);
    let (nodes, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let lineup = lineup_rows(&nodes);
    let mut groups = std::collections::HashSet::new();
    for row in &lineup {
        let position = row[1];
        let label = position.value().expect("the position's text");
        let byte = position_byte(label);
        groups.insert(position_colour(&Palette::DARK, byte));
        assert_eq!(
            painted_color(shape_of(&shapes, position)),
            position_colour(&Palette::DARK, byte),
            "{label}'s colour"
        );
    }
    assert_eq!(
        groups.len(),
        4,
        "premise: the rows hold all four groups (keeper, defence, midfield, attack)"
    );
}
