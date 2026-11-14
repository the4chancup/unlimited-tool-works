//! The formation pictures' rasterizer: the triangles epaint tessellates a
//! frame into, filled into an RGBA image as egui's GPU renderers fill a
//! window. Each pixel whose centre a triangle covers takes the triangle's
//! interpolated colour times its texture's texel, blended over what is
//! already there, all in premultiplied alpha and in the bytes' own
//! (gamma) space.

use std::collections::HashMap;
use std::sync::Arc;

use egui::epaint::textures::{TextureFilter, TextureWrapMode, TexturesDelta};
use egui::epaint::{ClippedPrimitive, ImageData, Primitive, Vertex};
use image::RgbaImage;

/// A texture as the frame's deltas leave it.
struct Texture {
    /// Its texels, premultiplied sRGBA.
    image: egui::ColorImage,
    /// How it is sampled.
    options: egui::TextureOptions,
}

/// `primitives`, tessellated at one pixel per point, filled into a
/// `width` by `height` image. `textures` is the frame's texture changes:
/// every `set` delta is applied in order (whole images, then the patches
/// on them, the font atlas among them), and the delta is left empty.
///
/// The image starts transparent and holds premultiplied alpha, which
/// equals plain alpha wherever the pixels are opaque: a picture paints
/// its opaque background first, so all of it is.
pub(super) fn rasterize(
    width: u32,
    height: u32,
    primitives: &[ClippedPrimitive],
    textures: &mut TexturesDelta,
) -> RgbaImage {
    let textures = apply(textures);
    let mut target = RgbaImage::new(width, height);
    for primitive in primitives {
        let mesh = match &primitive.primitive {
            Primitive::Mesh(mesh) => mesh,
            Primitive::Callback(_) => {
                unreachable!("a formation picture paints no custom-painted shape")
            }
        };
        let texture = textures
            .get(&mesh.texture_id)
            .expect("every texture a frame paints with is in its deltas");
        // The scissor rect as the renderers round it: whole pixels, inside
        // the image.
        let clip = primitive.clip_rect;
        let clip = [
            clip.min.x.round().clamp(0.0, width as f32) as u32,
            clip.min.y.round().clamp(0.0, height as f32) as u32,
            clip.max.x.round().clamp(0.0, width as f32) as u32,
            clip.max.y.round().clamp(0.0, height as f32) as u32,
        ];
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let corners = triangle.map(|index| &mesh.vertices[index as usize]);
            fill_triangle(&mut target, clip, texture, corners);
        }
    }
    target
}

/// Every texture `delta` sets, by id, its deltas applied in order; the
/// delta is left empty.
fn apply(delta: &mut TexturesDelta) -> HashMap<egui::TextureId, Texture> {
    let mut textures: HashMap<egui::TextureId, Texture> = HashMap::new();
    for (id, deltas) in std::mem::take(&mut delta.set) {
        for delta in deltas {
            let ImageData::Color(image) = delta.image;
            match delta.pos {
                None => {
                    textures.insert(
                        id,
                        Texture {
                            image: Arc::unwrap_or_clone(image),
                            options: delta.options,
                        },
                    );
                }
                Some([left, top]) => {
                    let texture = textures
                        .get_mut(&id)
                        .expect("a fresh context's first delta of a texture is its whole image");
                    let width = texture.image.size[0];
                    for (row, patch_row) in image.pixels.chunks_exact(image.size[0]).enumerate() {
                        let start = (top + row) * width + left;
                        texture.image.pixels[start..start + patch_row.len()]
                            .copy_from_slice(patch_row);
                    }
                    texture.options = delta.options;
                }
            }
        }
    }
    // The textures the frame freed are freed with the context, which the
    // picture drops.
    delta.clear();
    textures
}

/// Twice the signed area of the triangle `a`, `b`, `p`: positive when `p`
/// lies on the left of the edge from `a` to `b` (y pointing down).
fn edge(a: egui::Pos2, b: egui::Pos2, p: egui::Pos2) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

/// Whether the edge from `from` to `to` of a triangle wound with a
/// positive `edge` is a top or a left one, which owns the pixel centres
/// lying exactly on it: two triangles sharing an edge then fill each such
/// pixel once, not twice (a half-transparent shape would show the seam).
fn owns_its_centres(from: egui::Pos2, to: egui::Pos2) -> bool {
    let d = to - from;
    (d.y == 0.0 && d.x > 0.0) || d.y < 0.0
}

/// The pixels, from `first` up to `end` excluded, that a triangle whose
/// corners lie at `coordinates` along one axis may cover.
fn covered(coordinates: [f32; 3], first: u32, end: u32) -> std::ops::Range<u32> {
    let least = coordinates.into_iter().fold(f32::INFINITY, f32::min);
    let most = coordinates.into_iter().fold(f32::NEG_INFINITY, f32::max);
    // `as` saturates: a coordinate left of or above the image is pixel 0.
    (least.floor() as u32).max(first)..(most.ceil() as u32).min(end)
}

/// One triangle filled into `target`, inside `clip` (left, top, right,
/// bottom, the right and bottom excluded).
fn fill_triangle(
    target: &mut RgbaImage,
    clip: [u32; 4],
    texture: &Texture,
    vertices: [&Vertex; 3],
) {
    let [a, mut b, mut c] = vertices;
    let mut area = edge(a.pos, b.pos, c.pos);
    if area == 0.0 {
        return;
    }
    if area < 0.0 {
        std::mem::swap(&mut b, &mut c);
        area = -area;
    }
    // Texels per pixel decide the filter, as the GPU picks between
    // magnifying and minifying.
    let [texture_width, texture_height] = texture.image.size;
    let texel_area = edge(a.uv, b.uv, c.uv).abs() * (texture_width * texture_height) as f32;
    let filter = if texel_area > area {
        texture.options.minification
    } else {
        texture.options.magnification
    };
    let rows = covered([a.pos.y, b.pos.y, c.pos.y], clip[1], clip[3]);
    let columns = covered([a.pos.x, b.pos.x, c.pos.x], clip[0], clip[2]);
    let width = target.width();
    let pixels: &mut [u8] = target.as_mut();
    for y in rows {
        for x in columns.clone() {
            let centre = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
            // Each corner's weight is the edge facing it, seen from the
            // centre.
            let weights = [
                (edge(b.pos, c.pos, centre), b.pos, c.pos),
                (edge(c.pos, a.pos, centre), c.pos, a.pos),
                (edge(a.pos, b.pos, centre), a.pos, b.pos),
            ];
            let inside = weights.iter().all(|&(weight, from, to)| {
                weight > 0.0 || (weight == 0.0 && owns_its_centres(from, to))
            });
            if !inside {
                continue;
            }
            let [wa, wb, wc] = weights.map(|(weight, _, _)| weight / area);
            let colour = [a, b, c]
                .map(|vertex| vertex.color.to_array().map(f32::from))
                .into_iter()
                .zip([wa, wb, wc])
                .fold([0.0f32; 4], |sum, (colour, weight)| {
                    std::array::from_fn(|channel| sum[channel] + colour[channel] * weight)
                });
            let uv = egui::pos2(
                a.uv.x * wa + b.uv.x * wb + c.uv.x * wc,
                a.uv.y * wa + b.uv.y * wb + c.uv.y * wc,
            );
            let texel = sample(texture, uv, filter);
            let source: [f32; 4] =
                std::array::from_fn(|channel| colour[channel] * texel[channel] / 255.0);
            let at = ((y * width + x) * 4) as usize;
            let pixel = &mut pixels[at..at + 4];
            let keep = 1.0 - source[3] / 255.0;
            for (channel, byte) in pixel.iter_mut().enumerate() {
                *byte = (source[channel] + f32::from(*byte) * keep)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// `texture` sampled at `uv` through `filter`: the texel the point falls
/// in, or the four nearest texel centres weighted by distance; a texel
/// past an edge is found as the texture's wrap mode says.
fn sample(texture: &Texture, uv: egui::Pos2, filter: TextureFilter) -> [f32; 4] {
    let [width, height] = texture.image.size;
    let wrap = texture.options.wrap_mode;
    let texel = |x: f32, y: f32| {
        let x = wrapped(x as i64, width, wrap);
        let y = wrapped(y as i64, height, wrap);
        texture.image.pixels[y * width + x]
            .to_array()
            .map(f32::from)
    };
    let x = uv.x * width as f32;
    let y = uv.y * height as f32;
    match filter {
        TextureFilter::Nearest => texel(x.floor(), y.floor()),
        TextureFilter::Linear => {
            let (x, y) = (x - 0.5, y - 0.5);
            let (left, top) = (x.floor(), y.floor());
            let (across, down) = (x - left, y - top);
            let lerp = |from: [f32; 4], to: [f32; 4], t: f32| -> [f32; 4] {
                std::array::from_fn(|channel| from[channel] + (to[channel] - from[channel]) * t)
            };
            lerp(
                lerp(texel(left, top), texel(left + 1.0, top), across),
                lerp(texel(left, top + 1.0), texel(left + 1.0, top + 1.0), across),
                down,
            )
        }
    }
}

/// The texel index `index` reaches along an axis `size` texels long,
/// wrapped as `mode` says.
fn wrapped(index: i64, size: usize, mode: TextureWrapMode) -> usize {
    let size = size as i64;
    let index = match mode {
        TextureWrapMode::ClampToEdge => index.clamp(0, size - 1),
        TextureWrapMode::Repeat => index.rem_euclid(size),
        TextureWrapMode::MirroredRepeat => {
            let folded = index.rem_euclid(2 * size);
            if folded < size {
                folded
            } else {
                2 * size - 1 - folded
            }
        }
    };
    index as usize
}

#[cfg(test)]
mod tests {
    use egui::epaint::{ImageDelta, Mesh};
    use egui::{Color32, Rect, TextureId, TextureOptions, pos2};

    use super::*;

    /// The id of a 1 x 1 white texture, which a plain fill samples.
    const WHITE: TextureId = TextureId::User(0);
    /// The id of the textured tests' 2 x 2 texture.
    const CHECKER: TextureId = TextureId::User(1);
    /// An opaque background colour, under the blending tests.
    const GROUND: Color32 = Color32::from_rgb(40, 80, 120);

    /// The frame's deltas: `WHITE`, and `CHECKER` under `options`: red,
    /// green over blue, white.
    fn textures(options: TextureOptions) -> TexturesDelta {
        let mut delta = TexturesDelta::default();
        delta.push(
            WHITE,
            ImageDelta::full(
                egui::ColorImage::new([1, 1], vec![Color32::WHITE]),
                TextureOptions::NEAREST,
            ),
        );
        let checker = vec![
            Color32::from_rgb(255, 0, 0),
            Color32::from_rgb(0, 255, 0),
            Color32::from_rgb(0, 0, 255),
            Color32::WHITE,
        ];
        delta.push(
            CHECKER,
            ImageDelta::full(egui::ColorImage::new([2, 2], checker), options),
        );
        delta
    }

    /// `rect` filled in `colour` over `texture`, all of it from corner to
    /// corner, unclipped.
    fn rect(rect: Rect, colour: Color32, texture: TextureId) -> ClippedPrimitive {
        let mut mesh = Mesh::with_texture(texture);
        mesh.add_rect_with_uv(
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            colour,
        );
        ClippedPrimitive {
            clip_rect: Rect::EVERYTHING,
            primitive: Primitive::Mesh(mesh),
        }
    }

    /// `primitives` filled into an 8 x 8 image with `options` on the
    /// checker.
    fn draw(primitives: &[ClippedPrimitive], options: TextureOptions) -> RgbaImage {
        rasterize(8, 8, primitives, &mut textures(options))
    }

    /// A rect's pixels are exactly its colour, and the ones around it keep
    /// the image's transparent start.
    #[test]
    fn a_filled_rect_is_its_colour_and_nothing_else() {
        let colour = Color32::from_rgb(200, 100, 50);
        let filled = Rect::from_min_max(pos2(2.0, 2.0), pos2(5.0, 6.0));
        let image = draw(&[rect(filled, colour, WHITE)], TextureOptions::NEAREST);
        for (x, y, pixel) in image.enumerate_pixels() {
            let expected = if (2..5).contains(&x) && (2..6).contains(&y) {
                colour.to_array()
            } else {
                [0; 4]
            };
            assert_eq!(pixel.0, expected, "({x}, {y})");
        }
    }

    /// A half-transparent rect blends as the renderers blend:
    /// premultiplied source plus the ground times one minus its alpha.
    #[test]
    fn a_half_transparent_rect_blends_over_the_ground() {
        let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(8.0, 8.0));
        let veil = Color32::from_rgba_premultiplied(100, 50, 0, 128);
        let image = draw(
            &[rect(whole, GROUND, WHITE), rect(whole, veil, WHITE)],
            TextureOptions::NEAREST,
        );
        // 100 + 40 x 127/255, 50 + 80 x 127/255, 0 + 120 x 127/255,
        // 128 + 255 x 127/255.
        assert_eq!(image.get_pixel(3, 3).0, [120, 90, 60, 255]);
    }

    /// A textured quad samples its texture as its options say: the
    /// nearest texel, or the four nearest centres weighted (the edge
    /// clamped); magnified it takes the magnification filter, shrunk the
    /// minification one.
    #[test]
    fn a_textured_quad_samples_its_texture() {
        let quad = Rect::from_min_max(pos2(0.0, 0.0), pos2(4.0, 4.0));
        let point = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let shrink_linear = TextureOptions {
            magnification: TextureFilter::Nearest,
            minification: TextureFilter::Linear,
            ..TextureOptions::NEAREST
        };
        for (case, options, drawn, x, y, expected) in [
            (
                "nearest, top left",
                TextureOptions::NEAREST,
                quad,
                0,
                0,
                [255, 0, 0, 255],
            ),
            (
                "nearest, top right",
                TextureOptions::NEAREST,
                quad,
                3,
                0,
                [0, 255, 0, 255],
            ),
            (
                "nearest, bottom left",
                TextureOptions::NEAREST,
                quad,
                0,
                3,
                [0, 0, 255, 255],
            ),
            (
                "nearest, bottom right",
                TextureOptions::NEAREST,
                quad,
                3,
                3,
                [255; 4],
            ),
            (
                "nearest, inner",
                TextureOptions::NEAREST,
                quad,
                1,
                1,
                [255, 0, 0, 255],
            ),
            (
                "linear, clamped corner",
                TextureOptions::LINEAR,
                quad,
                0,
                0,
                [255, 0, 0, 255],
            ),
            (
                "linear, inner",
                TextureOptions::LINEAR,
                quad,
                1,
                1,
                [159, 64, 64, 255],
            ),
            (
                "linear, right of centre",
                TextureOptions::LINEAR,
                quad,
                2,
                1,
                [96, 191, 64, 255],
            ),
            ("magnified", shrink_linear, quad, 1, 1, [255, 0, 0, 255]),
            ("minified", shrink_linear, point, 0, 0, [128, 128, 128, 255]),
        ] {
            let image = draw(&[rect(drawn, Color32::WHITE, CHECKER)], options);
            assert_eq!(image.get_pixel(x, y).0, expected, "{case}");
        }
    }

    /// A pixel is the triangle's when its centre is inside: one the edge
    /// crosses with its centre outside stays as it was. Two triangles
    /// sharing an edge fill the centres on it once, so a half-transparent
    /// square split along its diagonal blends every pixel once.
    #[test]
    fn a_triangle_fills_the_pixels_whose_centre_it_covers() {
        // Triangles over `WHITE` in one colour, by their corners' indices
        // into `corners`.
        let mesh = |corners: &[egui::Pos2], triangles: &[[u32; 3]], colour: Color32| {
            let mut mesh = Mesh::with_texture(WHITE);
            for &pos in corners {
                mesh.vertices.push(Vertex {
                    pos,
                    uv: pos2(0.5, 0.5),
                    color: colour,
                });
            }
            for &[a, b, c] in triangles {
                mesh.add_triangle(a, b, c);
            }
            ClippedPrimitive {
                clip_rect: Rect::EVERYTHING,
                primitive: Primitive::Mesh(mesh),
            }
        };
        let corners = [pos2(0.0, 0.0), pos2(8.0, 0.0), pos2(0.0, 4.0)];
        let triangle = mesh(&corners, &[[0, 1, 2]], Color32::WHITE);
        let image = draw(&[triangle], TextureOptions::NEAREST);
        // The edge is x + 2y = 8: pixel (4, 1)'s centre gives 7.5, (5, 1)'s
        // 8.5 although its top-left corner, 7, is inside.
        assert_eq!(image.get_pixel(4, 1).0, [255; 4], "centre inside");
        assert_eq!(image.get_pixel(5, 1).0, [0; 4], "centre outside");

        let veil = Color32::from_rgba_premultiplied(100, 50, 0, 128);
        let corners = [
            pos2(2.0, 2.0),
            pos2(6.0, 2.0),
            pos2(6.0, 6.0),
            pos2(2.0, 6.0),
        ];
        let square = mesh(&corners, &[[0, 1, 2], [0, 2, 3]], veil);
        let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(8.0, 8.0));
        let image = draw(
            &[rect(whole, GROUND, WHITE), square],
            TextureOptions::NEAREST,
        );
        for (x, y, pixel) in image.enumerate_pixels() {
            let expected = if (2..6).contains(&x) && (2..6).contains(&y) {
                [120, 90, 60, 255]
            } else {
                GROUND.to_array()
            };
            assert_eq!(pixel.0, expected, "({x}, {y})");
        }
    }
}
