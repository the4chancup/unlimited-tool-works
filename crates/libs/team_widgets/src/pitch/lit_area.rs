//! The lit area: while a starter's marker is held, the pitch lights the
//! areas of the positions he is rated A or B at and darkens the rest to
//! 40% (the turf and its lines; the markers are drawn over it), as the
//! game's Edit Position screen does. It says where he is rated, not where
//! his position changes: the drop rule decides that, from every
//! position's area and his overalls.

use studio_core::theme::Palette;

use super::drop_rule::{self, AREAS_A};
use super::{SPOT_X0, SPOT_XS, SPOT_Y0, SPOT_YS};

/// The canvas rectangles a starter rated at `rated` (indexed by the
/// position byte) lights: each rated position's `AREAS_A` row through
/// `area_rect`.
pub(super) fn lit_rects(canvas: egui::Rect, rated: &[bool; 13]) -> Vec<egui::Rect> {
    (0..AREAS_A.len())
        .filter(|&position| rated[position])
        .map(|position| area_rect(canvas, AREAS_A[position]))
        .collect()
}

/// Darkens `canvas` outside the areas a starter rated at `rated` lights.
/// The darkened region is painted as cells that never overlap, since two
/// alpha fills over one place would read darker than the rest: the
/// lit rectangles' edges cut the canvas into a grid, and a cell is filled
/// when its centre lies in no lit rectangle. Lit rectangles may overlap
/// (a band's area holds the smaller ones inside it), since a cell is
/// filled once, or not at all, whatever covers it. A starter rated
/// nowhere (an empty roster slot, or a host that reads no ratings)
/// darkens nothing, rather than the whole pitch.
pub(super) fn paint_dim(
    painter: &egui::Painter,
    canvas: egui::Rect,
    rated: &[bool; 13],
    palette: &Palette,
) {
    if !rated.contains(&true) {
        return;
    }
    let lit = lit_rects(canvas, rated);
    let mut xs = vec![canvas.left(), canvas.right()];
    let mut ys = vec![canvas.top(), canvas.bottom()];
    for rect in &lit {
        xs.extend([rect.left(), rect.right()]);
        ys.extend([rect.top(), rect.bottom()]);
    }
    for edges in [&mut xs, &mut ys] {
        edges.sort_by(f32::total_cmp);
        edges.dedup();
    }
    for x in xs.windows(2) {
        for y in ys.windows(2) {
            let cell = egui::Rect::from_min_max(egui::pos2(x[0], y[0]), egui::pos2(x[1], y[1]));
            if !lit.iter().any(|rect| rect.contains(cell.center())) {
                painter.rect_filled(cell, 0, palette.lit_dim);
            }
        }
    }
}

/// An area's table `row` on the canvas: each edge half a unit out from
/// the row through the spot mapping, so two neighbouring rows meet
/// halfway between their last units. A row at the table's edge (x 5 or
/// 99, y 47) reaches the canvas' edge instead, as the game's lit panels
/// do, and the result is clipped to the canvas.
fn area_rect(canvas: egui::Rect, row: drop_rule::Rect) -> egui::Rect {
    let x = |units: f32| canvas.left() + SPOT_X0 + SPOT_XS * units;
    let y = |units: f32| canvas.top() + SPOT_Y0 - SPOT_YS * units;
    let left = if row.x_min <= 5 {
        canvas.left()
    } else {
        x(f32::from(row.x_min) - 0.5)
    };
    let right = if row.x_max >= 99 {
        canvas.right()
    } else {
        x(f32::from(row.x_max) + 0.5)
    };
    let top = if row.y_max >= 47 {
        canvas.top()
    } else {
        y(f32::from(row.y_max) + 0.5)
    };
    let bottom = y(f32::from(row.y_min) - 0.5);
    egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom)).intersect(canvas)
}
