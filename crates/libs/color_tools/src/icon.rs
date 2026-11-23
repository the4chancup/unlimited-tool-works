//! A kit's menu icon: one of the 24 two-color patterns its `icon_<N>` marker picks for the
//! prematch screens, recreated as egui shapes so the suite can draw it in the kit's own menu
//! colors at any size.

use egui::{Color32, CornerRadius, Rect, Shape, Stroke};

/// A pattern polygon in unit coordinates: x runs 0 (left) to 1 (right), y 0 (top) to
/// 1 (bottom).
type Polygon = &'static [[f32; 2]];

/// One menu icon pattern: which of the two menu colors fills the rectangle, and the convex
/// polygons drawn over it in the other color.
struct Pattern {
    /// The index into the kit's `[first, second]` menu colors of the pattern's ground color.
    ground: usize,
    /// The convex polygons drawn over the ground in the other menu color, in order.
    polygons: &'static [Polygon],
}

/// The centre stripe: the middle third, full height.
const CENTER_STRIPE: Polygon = &[
    [1.0 / 3.0, 0.0],
    [2.0 / 3.0, 0.0],
    [2.0 / 3.0, 1.0],
    [1.0 / 3.0, 1.0],
];
/// The chest band: a band across the chest, full width.
const CHEST_BAND: Polygon = &[[0.0, 0.25], [1.0, 0.25], [1.0, 0.55], [0.0, 0.55]];
/// The sash running from the top right to the bottom left.
const SASH_TOP_RIGHT: Polygon = &[
    [0.7, 0.0],
    [1.0, 0.0],
    [1.0, 0.3],
    [0.3, 1.0],
    [0.0, 1.0],
    [0.0, 0.7],
];
/// The sash running from the top left to the bottom right.
const SASH_TOP_LEFT: Polygon = &[
    [0.0, 0.0],
    [0.3, 0.0],
    [1.0, 0.7],
    [1.0, 1.0],
    [0.7, 1.0],
    [0.0, 0.3],
];
/// The V's left arm, from the top left shoulder down to the chest's middle.
const V_LEFT: Polygon = &[[0.0, 0.0], [0.3, 0.0], [0.5, 0.4], [0.5, 0.7]];
/// The V's right arm, from the top right shoulder down to the chest's middle.
const V_RIGHT: Polygon = &[[0.5, 0.4], [0.7, 0.0], [1.0, 0.0], [0.5, 0.7]];
/// The V's two arms.
const V_ARMS: &[Polygon] = &[V_LEFT, V_RIGHT];
/// The cross: the centre stripe over the chest band.
const CROSS: &[Polygon] = &[CENTER_STRIPE, CHEST_BAND];

/// The 24 icon patterns in `icon_<N>` order: 0 to 17 on the kit's first menu color, 18 to 23
/// the colour-swapped twins of 3, 4, 5, 6, 9 and 17 on the second.
const PATTERNS: [Pattern; 24] = [
    // 0: plain.
    Pattern {
        ground: 0,
        polygons: &[],
    },
    // 1: three vertical stripes.
    Pattern {
        ground: 0,
        polygons: &[
            &[[0.0, 0.0], [0.2, 0.0], [0.2, 1.0], [0.0, 1.0]],
            &[[0.4, 0.0], [0.6, 0.0], [0.6, 1.0], [0.4, 1.0]],
            &[[0.8, 0.0], [1.0, 0.0], [1.0, 1.0], [0.8, 1.0]],
        ],
    },
    // 2: three horizontal hoops.
    Pattern {
        ground: 0,
        polygons: &[
            &[[0.0, 0.0], [1.0, 0.0], [1.0, 0.2], [0.0, 0.2]],
            &[[0.0, 0.4], [1.0, 0.4], [1.0, 0.6], [0.0, 0.6]],
            &[[0.0, 0.8], [1.0, 0.8], [1.0, 1.0], [0.0, 1.0]],
        ],
    },
    // 3: the centre stripe.
    Pattern {
        ground: 0,
        polygons: &[CENTER_STRIPE],
    },
    // 4: the chest band.
    Pattern {
        ground: 0,
        polygons: &[CHEST_BAND],
    },
    // 5: the sash from the top right.
    Pattern {
        ground: 0,
        polygons: &[SASH_TOP_RIGHT],
    },
    // 6: the sash from the top left.
    Pattern {
        ground: 0,
        polygons: &[SASH_TOP_LEFT],
    },
    // 7: the left half.
    Pattern {
        ground: 0,
        polygons: &[&[[0.0, 0.0], [0.5, 0.0], [0.5, 1.0], [0.0, 1.0]]],
    },
    // 8: the right half.
    Pattern {
        ground: 0,
        polygons: &[&[[0.5, 0.0], [1.0, 0.0], [1.0, 1.0], [0.5, 1.0]]],
    },
    // 9: the V's two arms.
    Pattern {
        ground: 0,
        polygons: V_ARMS,
    },
    // 10: a 4 by 4 checkerboard, starting on the other color.
    Pattern {
        ground: 0,
        polygons: &[
            &[[0.0, 0.0], [0.25, 0.0], [0.25, 0.25], [0.0, 0.25]],
            &[[0.5, 0.0], [0.75, 0.0], [0.75, 0.25], [0.5, 0.25]],
            &[[0.25, 0.25], [0.5, 0.25], [0.5, 0.5], [0.25, 0.5]],
            &[[0.75, 0.25], [1.0, 0.25], [1.0, 0.5], [0.75, 0.5]],
            &[[0.0, 0.5], [0.25, 0.5], [0.25, 0.75], [0.0, 0.75]],
            &[[0.5, 0.5], [0.75, 0.5], [0.75, 0.75], [0.5, 0.75]],
            &[[0.25, 0.75], [0.5, 0.75], [0.5, 1.0], [0.25, 1.0]],
            &[[0.75, 0.75], [1.0, 0.75], [1.0, 1.0], [0.75, 1.0]],
        ],
    },
    // 11: the bottom half.
    Pattern {
        ground: 0,
        polygons: &[&[[0.0, 0.5], [1.0, 0.5], [1.0, 1.0], [0.0, 1.0]]],
    },
    // 12: the top half.
    Pattern {
        ground: 0,
        polygons: &[&[[0.0, 0.0], [1.0, 0.0], [1.0, 0.5], [0.0, 0.5]]],
    },
    // 13: the lower-left triangle.
    Pattern {
        ground: 0,
        polygons: &[&[[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]]],
    },
    // 14: the lower-right triangle.
    Pattern {
        ground: 0,
        polygons: &[&[[1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]],
    },
    // 15: the upper-left triangle.
    Pattern {
        ground: 0,
        polygons: &[&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]],
    },
    // 16: the upper-right triangle.
    Pattern {
        ground: 0,
        polygons: &[&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]],
    },
    // 17: the cross.
    Pattern {
        ground: 0,
        polygons: CROSS,
    },
    // 18: the centre stripe, colours swapped.
    Pattern {
        ground: 1,
        polygons: &[CENTER_STRIPE],
    },
    // 19: the chest band, colours swapped.
    Pattern {
        ground: 1,
        polygons: &[CHEST_BAND],
    },
    // 20: the sash from the top right, colours swapped.
    Pattern {
        ground: 1,
        polygons: &[SASH_TOP_RIGHT],
    },
    // 21: the sash from the top left, colours swapped.
    Pattern {
        ground: 1,
        polygons: &[SASH_TOP_LEFT],
    },
    // 22: the V's two arms, colours swapped.
    Pattern {
        ground: 1,
        polygons: V_ARMS,
    },
    // 23: the cross, colours swapped.
    Pattern {
        ground: 1,
        polygons: CROSS,
    },
];

/// The shapes that draw kit menu icon `icon`'s pattern over `rect`, in the kit's two menu
/// colors `[first, second]`: `rect` filled with the pattern's ground color, then its polygons
/// in the other color. Flat fills stretched to `rect`, no shirt outline.
///
/// `icon` is the kit's `icon_<N>` marker, 0 to 23.
///
/// # Panics
///
/// Panics when `icon` is past 23: the export's validation reports such a marker
/// (`kit_icon_invalid`) and leaves the kit with no icon, so the caller draws the default 3.
pub fn pattern_shapes(rect: Rect, icon: u8, colors: [Color32; 2]) -> Vec<Shape> {
    let pattern = &PATTERNS[usize::from(icon)];
    let mut shapes = Vec::with_capacity(pattern.polygons.len() + 1);
    shapes.push(Shape::rect_filled(
        rect,
        CornerRadius::ZERO,
        colors[pattern.ground],
    ));
    let other = colors[1 - pattern.ground];
    for polygon in pattern.polygons {
        let points = polygon
            .iter()
            .map(|&point| rect.lerp_inside(point))
            .collect();
        shapes.push(Shape::convex_polygon(points, other, Stroke::NONE));
    }
    shapes
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, pos2};

    const C1: Color32 = Color32::RED;
    const C2: Color32 = Color32::WHITE;

    /// The color of the last shape covering `pos`; panics when none does.
    fn color_at(shapes: &[Shape], pos: Pos2) -> Color32 {
        /// Whether the convex polygon covers `pos`: on the same side of every edge, edge
        /// points included.
        fn covers(polygon: &[Pos2], pos: Pos2) -> bool {
            let mut side = 0.0f32;
            for (i, a) in polygon.iter().enumerate() {
                let b = polygon[(i + 1) % polygon.len()];
                let cross = (b.x - a.x) * (pos.y - a.y) - (b.y - a.y) * (pos.x - a.x);
                if cross == 0.0 {
                    continue;
                }
                if side == 0.0 {
                    side = cross.signum();
                } else if cross.signum() != side {
                    return false;
                }
            }
            true
        }

        for shape in shapes.iter().rev() {
            match shape {
                Shape::Rect(rect) if rect.rect.contains(pos) => return rect.fill,
                Shape::Path(path) if covers(&path.points, pos) => return path.fill,
                _ => {}
            }
        }
        panic!("no shape covers {pos:?}");
    }

    /// One icon's expected samples: `(unit x, unit y, expected color)` rows.
    type Samples = &'static [(f32, f32, Color32)];

    /// Every icon, sampled where each color shows over the unit square.
    #[test]
    fn every_pattern_sampled() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        let cases: &[(u8, Samples)] = &[
            (0, &[(0.5, 0.5, C1), (0.1, 0.1, C1), (0.9, 0.9, C1)]),
            (
                1,
                &[
                    (0.3, 0.5, C1),
                    (0.7, 0.5, C1),
                    (0.1, 0.5, C2),
                    (0.5, 0.5, C2),
                    (0.9, 0.5, C2),
                ],
            ),
            (
                2,
                &[
                    (0.5, 0.3, C1),
                    (0.5, 0.7, C1),
                    (0.5, 0.1, C2),
                    (0.5, 0.5, C2),
                    (0.5, 0.9, C2),
                ],
            ),
            (
                3,
                &[
                    (0.2, 0.5, C1),
                    (0.8, 0.5, C1),
                    (0.5, 0.1, C2),
                    (0.5, 0.9, C2),
                ],
            ),
            (
                4,
                &[
                    (0.5, 0.1, C1),
                    (0.5, 0.8, C1),
                    (0.5, 0.4, C2),
                    (0.1, 0.4, C2),
                ],
            ),
            (
                5,
                &[
                    (0.1, 0.1, C1),
                    (0.9, 0.9, C1),
                    (0.5, 0.5, C2),
                    (0.9, 0.1, C2),
                    (0.1, 0.9, C2),
                ],
            ),
            (
                6,
                &[
                    (0.9, 0.1, C1),
                    (0.1, 0.9, C1),
                    (0.5, 0.5, C2),
                    (0.1, 0.1, C2),
                    (0.9, 0.9, C2),
                ],
            ),
            (7, &[(0.75, 0.5, C1), (0.25, 0.5, C2)]),
            (8, &[(0.25, 0.5, C1), (0.75, 0.5, C2)]),
            (
                9,
                &[
                    (0.5, 0.1, C1),
                    (0.5, 0.9, C1),
                    (0.1, 0.6, C1),
                    (0.1, 0.05, C2),
                    (0.9, 0.05, C2),
                    (0.5, 0.55, C2),
                ],
            ),
            (
                10,
                &[
                    (0.375, 0.125, C1),
                    (0.125, 0.375, C1),
                    (0.625, 0.875, C1),
                    (0.125, 0.125, C2),
                    (0.375, 0.375, C2),
                    (0.875, 0.875, C2),
                ],
            ),
            (11, &[(0.5, 0.25, C1), (0.5, 0.75, C2)]),
            (12, &[(0.5, 0.75, C1), (0.5, 0.25, C2)]),
            (13, &[(0.8, 0.2, C1), (0.2, 0.8, C2)]),
            (14, &[(0.2, 0.2, C1), (0.8, 0.8, C2)]),
            (15, &[(0.8, 0.8, C1), (0.2, 0.2, C2)]),
            (16, &[(0.2, 0.8, C1), (0.8, 0.2, C2)]),
            (
                17,
                &[
                    (0.1, 0.1, C1),
                    (0.9, 0.8, C1),
                    (0.5, 0.1, C2),
                    (0.1, 0.4, C2),
                    (0.9, 0.4, C2),
                    (0.5, 0.9, C2),
                ],
            ),
            (18, &[(0.5, 0.5, C1), (0.2, 0.5, C2), (0.8, 0.5, C2)]),
            (19, &[(0.5, 0.4, C1), (0.5, 0.1, C2), (0.5, 0.8, C2)]),
            (20, &[(0.5, 0.5, C1), (0.1, 0.1, C2), (0.9, 0.9, C2)]),
            (21, &[(0.5, 0.5, C1), (0.9, 0.1, C2), (0.1, 0.9, C2)]),
            (
                22,
                &[
                    (0.1, 0.05, C1),
                    (0.5, 0.55, C1),
                    (0.5, 0.1, C2),
                    (0.5, 0.9, C2),
                ],
            ),
            (
                23,
                &[
                    (0.5, 0.1, C1),
                    (0.1, 0.4, C1),
                    (0.1, 0.1, C2),
                    (0.9, 0.8, C2),
                ],
            ),
        ];
        let mut misses = Vec::new();
        for &(icon, samples) in cases {
            let shapes = pattern_shapes(rect, icon, [C1, C2]);
            for &(x, y, expected) in samples {
                let actual = color_at(&shapes, pos2(x * 100.0, y * 100.0));
                if actual != expected {
                    misses.push(format!(
                        "icon {icon} at ({x}, {y}): {actual:?} != {expected:?}"
                    ));
                }
            }
        }
        assert!(misses.is_empty(), "{}", misses.join("\n"));
    }

    /// The pattern stretches to whatever rectangle it is given: the grid's kit-cell strip.
    #[test]
    fn pattern_fills_the_rect_it_is_given() {
        let strip = Rect::from_min_max(pos2(100.0, 40.0), pos2(124.0, 49.0));

        // Icon 7, the left half: the ground is a Shape::Rect of exactly that rect in c1.
        let shapes = pattern_shapes(strip, 7, [C1, C2]);
        let Shape::Rect(ground) = &shapes[0] else {
            panic!("the ground is not a rect: {:?}", shapes[0]);
        };
        assert_eq!(ground.rect, strip);
        assert_eq!(ground.fill, C1);
        assert_eq!(color_at(&shapes, pos2(106.0, 44.5)), C2);
        assert_eq!(color_at(&shapes, pos2(118.0, 44.5)), C1);

        // Icon 2's hoops are a fifth of the strip's 9-point height each.
        let shapes = pattern_shapes(strip, 2, [C1, C2]);
        assert_eq!(color_at(&shapes, pos2(112.0, 40.9)), C2);
        assert_eq!(color_at(&shapes, pos2(112.0, 42.7)), C1);
    }

    /// The icon is drawn in the colors the caller passes, whoever they are.
    #[test]
    fn colors_are_the_callers() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        let custom = Color32::from_rgb(12, 34, 56);

        // Icon 0, the plain one: one shape, the rect, in the first color.
        let shapes = pattern_shapes(rect, 0, [custom, C2]);
        assert_eq!(shapes.len(), 1);
        assert_eq!(color_at(&shapes, pos2(50.0, 50.0)), custom);

        // Icon 18, the centre stripe on the second color: the ground is c2.
        let shapes = pattern_shapes(rect, 18, [C1, Color32::BLUE]);
        let Shape::Rect(ground) = &shapes[0] else {
            panic!("the ground is not a rect: {:?}", shapes[0]);
        };
        assert_eq!(ground.fill, Color32::BLUE);
    }
}
