//! Resampling straight-alpha RGBA8 pixels with Lanczos3, through the `image` crate the raster
//! decoders already use: the suite's one resampler (`spec/libs/dds_convert.md` "`dds_convert` API").

use image::imageops::{self, FilterType};
use image::{ImageBuffer, Rgba};

use crate::ConvertError;

/// `pixels` (straight-alpha RGBA8, `width` x `height`, row-major) resampled to `new_width` x
/// `new_height` with Lanczos3, each channel as it is (no premultiplication). Near an edge the
/// filter's taps that fall past it are left out and the remaining weights rescaled to sum to
/// one, so a flat image stays flat; a side whose size does not change is not filtered along.
///
/// # Errors
///
/// `ConvertError::InvalidDecoded` for a zero size, old or new, or a `pixels` whose length is
/// not `width * height * 4`.
pub fn resize(
    pixels: &[u8],
    width: u32,
    height: u32,
    new_width: u32,
    new_height: u32,
) -> Result<Vec<u8>, ConvertError> {
    if width == 0 || height == 0 || new_width == 0 || new_height == 0 {
        return Err(ConvertError::InvalidDecoded("zero dimension"));
    }
    let expected = u64::from(width) * u64::from(height) * 4;
    if pixels.len() as u64 != expected {
        return Err(ConvertError::InvalidDecoded("pixel buffer size"));
    }
    let source = ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(width, height, pixels)
        .ok_or(ConvertError::InvalidDecoded("pixel buffer size"))?;
    Ok(imageops::resize(&source, new_width, new_height, FilterType::Lanczos3).into_raw())
}

/// `pixels` (straight-alpha RGBA8, `width` x `height`, row-major) resampled to `new_width` x
/// `new_height` as [`resize`] does, but on premultiplied colour, and handed back straight. A
/// transparent texel still stores a colour (black, in most logos), which a straight resample
/// blends into its opaque neighbours as a dark fringe along the edge; premultiplied, it adds
/// nothing. Each colour is multiplied by its alpha over 255, resampled, held at most its alpha
/// (Lanczos3 overshoots near a sharp edge, and a premultiplied colour above its alpha is no
/// colour), then divided back; a texel left with alpha 0 gets colour 0. Both steps round, so a
/// partly transparent texel's colour can move by one.
///
/// # Errors
///
/// As [`resize`].
pub fn resize_straight_alpha(
    pixels: &[u8],
    width: u32,
    height: u32,
    new_width: u32,
    new_height: u32,
) -> Result<Vec<u8>, ConvertError> {
    let mut premultiplied = pixels.to_vec();
    for texel in premultiplied.as_chunks_mut::<4>().0 {
        let alpha = u32::from(texel[3]);
        for channel in &mut texel[..3] {
            *channel = rounded_ratio(u32::from(*channel) * alpha, 255);
        }
    }
    let mut resized = resize(&premultiplied, width, height, new_width, new_height)?;
    for texel in resized.as_chunks_mut::<4>().0 {
        let alpha = texel[3];
        for channel in &mut texel[..3] {
            *channel = if alpha == 0 {
                0
            } else {
                rounded_ratio(u32::from((*channel).min(alpha)) * 255, u32::from(alpha))
            };
        }
    }
    Ok(resized)
}

/// `numerator / denominator` rounded to the nearest integer, for a quotient of at most 255.
fn rounded_ratio(numerator: u32, denominator: u32) -> u8 {
    u8::try_from((numerator + denominator / 2) / denominator)
        .expect("a colour times its alpha over 255, or a colour held at most its alpha over it")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `width` x `height` image whose texel at (x, y) is `color(x, y)`.
    fn image(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&color(x, y));
            }
        }
        pixels
    }

    #[test]
    fn a_flat_image_stays_exactly_flat_at_any_size() {
        let color = [197, 81, 82, 200];
        let flat = image(20, 12, |_, _| color);
        for (new_width, new_height) in [(15, 12), (7, 3), (33, 40), (1, 1), (20, 5)] {
            let resized = resize(&flat, 20, 12, new_width, new_height).unwrap();
            assert_eq!(
                resized,
                image(new_width, new_height, |_, _| color),
                "{new_width}x{new_height}"
            );
        }
    }

    #[test]
    fn with_the_height_unchanged_rows_do_not_mix() {
        let row_color = |y: u32| [(y * 40) as u8, 255 - (y * 30) as u8, (y * 7) as u8, 255];
        let rows = image(16, 6, |_, y| row_color(y));
        for new_width in [12, 5, 29] {
            let resized = resize(&rows, 16, 6, new_width, 6).unwrap();
            assert_eq!(
                resized,
                image(new_width, 6, |_, y| row_color(y)),
                "{new_width}"
            );
        }
    }

    #[test]
    fn a_two_color_image_halved_in_width_keeps_each_color_on_its_side() {
        let (left, right) = ([255, 0, 0, 255], [0, 0, 255, 255]);
        let halves = image(32, 4, |x, _| if x < 16 { left } else { right });
        let resized = resize(&halves, 32, 4, 16, 4).unwrap();
        for (index, texel) in resized.as_chunks::<4>().0.iter().enumerate() {
            let x = index as u32 % 16;
            let (own, other) = if x < 8 { (left, right) } else { (right, left) };
            // Halving, the filter reaches 6 source texels either side: a texel whose reach
            // stays on its side is exactly its color, one whose reach crosses the boundary
            // blends the two and stays nearer its own.
            if x <= 4 || x >= 11 {
                assert_eq!(*texel, own, "x {x}");
            } else {
                let distance = |color: [u8; 4]| -> u32 {
                    (0..4).map(|c| u32::from(texel[c].abs_diff(color[c]))).sum()
                };
                assert!(distance(own) < distance(other), "x {x}: {texel:?}");
            }
        }
    }

    #[test]
    fn an_edge_against_transparent_black_stays_white_on_premultiplied_colour() {
        let halves = image(32, 8, |x, _| if x < 16 { [255; 4] } else { [0; 4] });
        let visible = |texels: &[u8]| -> Vec<[u8; 4]> {
            texels
                .as_chunks::<4>()
                .0
                .iter()
                .copied()
                .filter(|texel| texel[3] > 0)
                .collect()
        };
        let straight = resize(&halves, 32, 8, 12, 3).unwrap();
        assert!(
            visible(&straight)
                .iter()
                .any(|texel| texel[..3].iter().any(|&channel| channel < 200)),
            "premise: resampled straight, an edge texel darkens: {straight:?}"
        );
        let premultiplied = resize_straight_alpha(&halves, 32, 8, 12, 3).unwrap();
        assert!(
            visible(&premultiplied).iter().any(|texel| texel[3] < 255),
            "premise: a texel straddles the edge"
        );
        for texel in visible(&premultiplied) {
            assert!(
                texel[..3].iter().all(|&channel| channel >= 250),
                "{texel:?}"
            );
        }
    }

    #[test]
    fn an_overshoot_past_a_half_transparent_edge_is_held_at_full_colour() {
        // The same halves premultiplied by hand (128 × 255 / 255 and 0): Lanczos3 rings above
        // the white side's 128, past what its alpha allows.
        let premultiplied = image(32, 8, |x, _| {
            if x < 16 {
                [128, 128, 128, 128]
            } else {
                [0, 0, 0, 128]
            }
        });
        let rung = resize(&premultiplied, 32, 8, 12, 3).unwrap();
        assert!(
            rung.as_chunks::<4>()
                .0
                .iter()
                .any(|texel| texel[0] > texel[3]),
            "premise: a colour overshoots its alpha: {rung:?}"
        );
        let halves = image(32, 8, |x, _| {
            if x < 16 {
                [255, 255, 255, 128]
            } else {
                [0, 0, 0, 128]
            }
        });
        let resized = resize_straight_alpha(&halves, 32, 8, 12, 3).unwrap();
        let texels = resized.as_chunks::<4>().0;
        assert!(texels.iter().all(|texel| texel[3] == 128), "{resized:?}");
        assert_eq!(
            texels.iter().map(|texel| texel[0]).max(),
            Some(255),
            "{resized:?}"
        );
    }

    #[test]
    fn at_its_own_size_a_straight_picture_keeps_its_colours_and_clears_transparent_ones() {
        let texels = [[255, 128, 0, 128], [200, 100, 50, 0], [10, 20, 30, 255]];
        let picture = image(3, 1, |x, _| texels[x as usize]);
        assert_eq!(
            resize_straight_alpha(&picture, 3, 1, 3, 1).unwrap(),
            [255, 128, 0, 128, 0, 0, 0, 0, 10, 20, 30, 255]
        );
    }

    #[test]
    fn a_zero_size_or_a_buffer_of_the_wrong_length_is_invalid() {
        let pixels = image(4, 4, |_, _| [1, 2, 3, 4]);
        for (width, height, new_width, new_height) in
            [(0, 4, 2, 2), (4, 0, 2, 2), (4, 4, 0, 2), (4, 4, 2, 0)]
        {
            assert!(matches!(
                resize(&pixels, width, height, new_width, new_height),
                Err(ConvertError::InvalidDecoded("zero dimension"))
            ));
        }
        for (width, height) in [(4, 3), (4, 5)] {
            assert!(matches!(
                resize(&pixels, width, height, 2, 2),
                Err(ConvertError::InvalidDecoded("pixel buffer size"))
            ));
        }
    }
}
