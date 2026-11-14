//! Dominant kit-color extraction: where the Compiler's two menu colors come from when a
//! kit folder has no `colors.txt`. The community kit template has a fixed UV layout, so the
//! shirt and shorts sit in fixed normalized regions of `kit.dds`; the texture is sampled on a
//! stride, quantized and greedily clustered, and the two menu colors are picked from the
//! ranked clusters.

use std::collections::HashMap;

/// A normalized rectangle of the kit texture, `0.0..=1.0` on both axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Region {
    /// Left edge.
    pub x0: f32,
    /// Top edge.
    pub y0: f32,
    /// Right edge, exclusive.
    pub x1: f32,
    /// Bottom edge, exclusive.
    pub y1: f32,
}

/// The shirt band of the community kit template, inset from the measured zone.
pub(crate) const SHIRT: Region = Region {
    x0: 0.36,
    y0: 0.05,
    x1: 0.64,
    y1: 0.88,
};

/// The two shorts panels.
pub(crate) const SHORTS: [Region; 2] = [
    Region {
        x0: 0.04,
        y0: 0.61,
        x1: 0.29,
        y1: 0.90,
    },
    Region {
        x0: 0.71,
        y0: 0.61,
        x1: 0.96,
        y1: 0.90,
    },
];

/// One color cluster of a region: its mean color and its share of the region's samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cluster {
    /// The count-weighted mean color of the cluster's bins.
    pub color: [u8; 3],
    /// The cluster's share of the region's opaque samples, `0.0..=1.0`.
    pub share: f32,
}

/// The two menu colors derived from a kit texture, with the ranked clusters they came from.
#[derive(Debug, Clone, PartialEq)]
pub struct KitColors {
    /// The kit's main menu color: the shirt's dominant cluster.
    pub color1: [u8; 3],
    /// The kit's secondary menu color.
    pub color2: [u8; 3],
    /// Which candidate color 2 came from.
    pub color2_source: Color2Source,
    /// The shirt region's clusters, ranked largest first.
    pub shirt: Vec<Cluster>,
    /// The shorts regions' clusters (both panels clustered together), ranked largest first.
    pub shorts: Vec<Cluster>,
}

/// Where [`KitColors::color2`] was picked from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color2Source {
    /// The shirt's second cluster held at least a quarter of the region: a two-tone kit.
    ShirtSecond,
    /// The shorts' dominant color.
    Shorts,
    /// The shirt's second cluster held at least a tenth: a trim color (the shorts matched).
    ShirtTrim,
    /// The shorts' second cluster.
    ShortsSecond,
    /// Nothing distinct was found: color 2 is the shorts' dominant color, the same as or
    /// near-identical to color 1 — a one-color kit.
    SameAsColor1,
}

/// Every 4th pixel on both axes: 16k samples per region on a 2048 texture, plenty for a
/// two-color answer.
const STRIDE: u32 = 4;
/// Pixels with alpha below this carry no kit color.
const OPAQUE: u8 = 128;
/// Two 5-bit-quantized bins merge into one cluster when their means are within this RGB
/// distance — near-shades of one kit color, not two colors.
const MERGE_DISTANCE: f32 = 24.0;
/// Two colors count as distinct from this RGB distance up (about navy against black).
const DISTINCT_DISTANCE: f32 = 60.0;
/// A second shirt cluster at this share or more means a two-tone kit, not a trim.
const TWO_TONE_SHARE: f32 = 0.25;
/// A second shirt cluster at this share or more is a usable trim color when the shorts
/// match the shirt.
const TRIM_SHARE: f32 = 0.10;

/// One 5-bit color bin of a region's samples.
#[derive(Clone, Copy)]
struct Bin {
    /// The bin key: `(r >> 3) << 10 | (g >> 3) << 5 | b >> 3`.
    key: usize,
    /// How many samples fell in the bin.
    count: u64,
    /// Channel sums of the samples, for the bin mean.
    sum: [u64; 3],
}

/// RGB Euclidean distance between two colors.
fn distance(a: [u8; 3], b: [u8; 3]) -> f32 {
    let squared: i32 = (0..3)
        .map(|channel| {
            let difference = i32::from(a[channel]) - i32::from(b[channel]);
            difference * difference
        })
        .sum();
    (squared as f32).sqrt()
}

/// Whether two colors read as different kit colors to a viewer.
fn distinct(a: [u8; 3], b: [u8; 3]) -> bool {
    distance(a, b) >= DISTINCT_DISTANCE
}

/// The bins of a set of regions' samples: every `STRIDE`th pixel on both axes within each
/// region's pixel rectangle, alpha below `OPAQUE` skipped. A region smaller than the stride
/// still samples its first row and column.
fn sample(rgba: &[u8], width: u32, height: u32, regions: &[Region]) -> (Vec<Bin>, u64) {
    let mut bins: HashMap<usize, (u64, [u64; 3])> = HashMap::new();
    let mut total = 0u64;
    for region in regions {
        let x0 = (region.x0 * width as f32) as u32;
        let y0 = (region.y0 * height as f32) as u32;
        let x1 = (region.x1 * width as f32) as u32;
        let y1 = (region.y1 * height as f32) as u32;
        let mut y = y0;
        while y < y1 {
            let mut x = x0;
            while x < x1 {
                let start = (y as usize * width as usize + x as usize) * 4;
                if let Some(pixel) = rgba.get(start..start + 4)
                    && pixel[3] >= OPAQUE
                {
                    let key = (usize::from(pixel[0] >> 3) << 10)
                        | (usize::from(pixel[1] >> 3) << 5)
                        | usize::from(pixel[2] >> 3);
                    let entry = bins.entry(key).or_default();
                    entry.0 += 1;
                    for (sum, value) in entry.1.iter_mut().zip(&pixel[..3]) {
                        *sum += u64::from(*value);
                    }
                    total += 1;
                }
                x += STRIDE;
            }
            y += STRIDE;
        }
    }
    let mut sorted: Vec<Bin> = bins
        .into_iter()
        .map(|(key, (count, sum))| Bin { key, count, sum })
        .collect();
    sorted.sort_by(|a, b| b.count.cmp(&a.count).then(a.key.cmp(&b.key)));
    (sorted, total)
}

/// The count-weighted mean color of a bin's or cluster's channel sums.
fn mean_color(count: u64, sum: [u64; 3]) -> [u8; 3] {
    let channel = |sum: u64| ((sum + count / 2) / count) as u8;
    [channel(sum[0]), channel(sum[1]), channel(sum[2])]
}

/// Ranked clusters of a region set's samples: bins by count descending (ties by key, so the
/// order is deterministic), each merged into the first existing cluster whose mean lies
/// within `MERGE_DISTANCE`, else starting a new one.
fn cluster(rgba: &[u8], width: u32, height: u32, regions: &[Region]) -> Vec<Cluster> {
    // The buffer must hold exactly 4 * width * height bytes: a short one would cluster a
    // partial image and huge dimensions would overflow the pixel math in `sample`.
    if (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        != Some(rgba.len())
    {
        return Vec::new();
    }
    let (bins, total) = sample(rgba, width, height, regions);
    if total == 0 {
        return Vec::new();
    }
    let mut clusters: Vec<(u64, [u64; 3])> = Vec::new();
    for bin in bins {
        let bin_mean = mean_color(bin.count, bin.sum);
        match clusters
            .iter_mut()
            .find(|(count, sum)| distance(mean_color(*count, *sum), bin_mean) < MERGE_DISTANCE)
        {
            Some((count, sum)) => {
                *count += bin.count;
                for (sum, value) in sum.iter_mut().zip(bin.sum) {
                    *sum += value;
                }
            }
            None => clusters.push((bin.count, bin.sum)),
        }
    }
    let mut result: Vec<Cluster> = clusters
        .into_iter()
        .map(|(count, sum)| Cluster {
            color: mean_color(count, sum),
            share: count as f32 / total as f32,
        })
        .collect();
    result.sort_by(|a, b| {
        b.share
            .partial_cmp(&a.share)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.color.cmp(&b.color))
    });
    result
}

/// The kit's two menu colors; `None` when the shirt region has no opaque pixel.
/// `rgba.len()` must be exactly `4 * width * height`; anything else yields `None`.
pub fn extract_kit_colors(rgba: &[u8], width: u32, height: u32) -> Option<KitColors> {
    let shirt = cluster(rgba, width, height, &[SHIRT]);
    let shirt_dominant = *shirt.first()?;
    let shorts = cluster(rgba, width, height, &SHORTS);
    let color1 = shirt_dominant.color;
    let (color2, color2_source) = match (shirt.get(1), shorts.first(), shorts.get(1)) {
        (Some(second), _, _)
            if second.share >= TWO_TONE_SHARE && distinct(second.color, color1) =>
        {
            (second.color, Color2Source::ShirtSecond)
        }
        (_, Some(dominant), _) if distinct(dominant.color, color1) => {
            (dominant.color, Color2Source::Shorts)
        }
        (Some(second), _, _) if second.share >= TRIM_SHARE && distinct(second.color, color1) => {
            (second.color, Color2Source::ShirtTrim)
        }
        (_, _, Some(second)) if distinct(second.color, color1) => {
            (second.color, Color2Source::ShortsSecond)
        }
        (_, dominant, _) => (
            dominant.map_or(color1, |cluster| cluster.color),
            Color2Source::SameAsColor1,
        ),
    };
    Some(KitColors {
        color1,
        color2,
        color2_source,
        shirt,
        shorts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: u32 = 256;

    /// Ranked color clusters of one region of an RGBA8 texture. `rgba.len()` must be exactly
    /// `4 * width * height`; anything else yields an empty list, as does an empty region or a
    /// fully transparent image.
    fn dominant_colors(rgba: &[u8], width: u32, height: u32, region: Region) -> Vec<Cluster> {
        cluster(rgba, width, height, &[region])
    }

    /// A SIZE x SIZE texture whose pixel (x, y) is the RGBA `paint` returns for it.
    fn texture(width: u32, height: u32, paint: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&paint(x, y));
            }
        }
        rgba
    }

    /// The shirt region in pixels on a SIZE texture.
    fn shirt_px() -> (u32, u32, u32, u32) {
        (
            (SHIRT.x0 * SIZE as f32) as u32,
            (SHIRT.y0 * SIZE as f32) as u32,
            (SHIRT.x1 * SIZE as f32) as u32,
            (SHIRT.y1 * SIZE as f32) as u32,
        )
    }

    fn in_shirt(x: u32, y: u32) -> bool {
        let (x0, y0, x1, y1) = shirt_px();
        x >= x0 && x < x1 && y >= y0 && y < y1
    }

    fn in_shorts(x: u32, y: u32) -> bool {
        SHORTS.iter().any(|region| {
            let x0 = (region.x0 * SIZE as f32) as u32;
            let y0 = (region.y0 * SIZE as f32) as u32;
            let x1 = (region.x1 * SIZE as f32) as u32;
            let y1 = (region.y1 * SIZE as f32) as u32;
            x >= x0 && x < x1 && y >= y0 && y < y1
        })
    }

    const GREY: [u8; 4] = [128, 128, 128, 255];

    #[test]
    fn plain_shirt_and_shorts() {
        let red = [200, 30, 30];
        let blue = [20, 20, 90];
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                [red[0], red[1], red[2], 255]
            } else if in_shorts(x, y) {
                [blue[0], blue[1], blue[2], 255]
            } else {
                GREY
            }
        });
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.color1, red);
        assert_eq!(kit.color2, blue);
        assert_eq!(kit.color2_source, Color2Source::Shorts);
        assert!(kit.shirt[0].share > 0.99, "{}", kit.shirt[0].share);
    }

    #[test]
    fn two_tone_shirt() {
        let (x0, _, x1, _) = shirt_px();
        let mid = (x0 + x1) / 2;
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                if x < mid {
                    [200, 30, 30, 255]
                } else {
                    [255, 255, 255, 255]
                }
            } else if in_shorts(x, y) {
                [20, 20, 90, 255]
            } else {
                GREY
            }
        });
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.color1, [200, 30, 30]);
        assert_eq!(kit.color2, [255, 255, 255]);
        assert_eq!(kit.color2_source, Color2Source::ShirtSecond);
        for cluster in &kit.shirt {
            assert!((0.4..=0.6).contains(&cluster.share), "{}", cluster.share);
        }
    }

    #[test]
    fn trim_when_shorts_match() {
        let gold = [255, 200, 0];
        // A 4-px stripe per 28 px of band width: about one seventh of the shirt.
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                if x % 28 >= 24 {
                    [gold[0], gold[1], gold[2], 255]
                } else {
                    [0, 0, 0, 255]
                }
            } else {
                [0, 0, 0, 255]
            }
        });
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.color1, [0, 0, 0]);
        assert_eq!(kit.color2, gold);
        assert_eq!(kit.color2_source, Color2Source::ShirtTrim);
    }

    #[test]
    fn one_color_kit() {
        let rgba = texture(SIZE, SIZE, |_, _| [0, 0, 0, 255]);
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.color1, [0, 0, 0]);
        assert_eq!(kit.color2, kit.color1);
        assert_eq!(kit.color2_source, Color2Source::SameAsColor1);
    }

    #[test]
    fn logo_is_ignored() {
        let (x0, y0, _, _) = shirt_px();
        // A badge of about 5% of the band, on stride-aligned coordinates.
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                if (x0 + 8..x0 + 28).contains(&x) && (y0 + 8..y0 + 56).contains(&y) {
                    [255, 255, 255, 255]
                } else {
                    [200, 30, 30, 255]
                }
            } else if in_shorts(x, y) {
                [20, 20, 90, 255]
            } else {
                GREY
            }
        });
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.color1, [200, 30, 30]);
        assert_eq!(kit.shirt[1].color, [255, 255, 255]);
        assert!(kit.shirt[1].share < 0.10, "{}", kit.shirt[1].share);
        assert_eq!(kit.color2, [20, 20, 90]);
        assert_eq!(kit.color2_source, Color2Source::Shorts);
    }

    #[test]
    fn near_shades_merge() {
        let (x0, _, x1, _) = shirt_px();
        let mid = (x0 + x1) / 2;
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                if x < mid {
                    [200, 30, 30, 255]
                } else {
                    [210, 35, 35, 255]
                }
            } else {
                [20, 20, 90, 255]
            }
        });
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.shirt.len(), 1);
        assert!(kit.shirt[0].share > 0.99, "{}", kit.shirt[0].share);
        let mean = kit.shirt[0].color;
        assert!((200..=210).contains(&mean[0]), "{mean:?}");
        assert!((30..=35).contains(&mean[1]), "{mean:?}");
        assert!((30..=35).contains(&mean[2]), "{mean:?}");
    }

    #[test]
    fn transparent_pixels_are_skipped() {
        let (x0, y0, _, _) = shirt_px();
        // Only a 10-px opaque green square in the band; the rest is invisible.
        let rgba = texture(SIZE, SIZE, |x, y| {
            if (x0 + 4..x0 + 14).contains(&x) && (y0 + 4..y0 + 14).contains(&y) {
                [0, 180, 0, 255]
            } else {
                [0, 0, 0, 0]
            }
        });
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.color1, [0, 180, 0]);

        let invisible = texture(SIZE, SIZE, |_, _| [0, 0, 0, 0]);
        assert!(extract_kit_colors(&invisible, SIZE, SIZE).is_none());
        assert!(dominant_colors(&invisible, SIZE, SIZE, SHIRT).is_empty());
    }

    #[test]
    fn small_and_short_inputs() {
        let rgba = texture(4, 4, |_, _| [90, 0, 0, 255]);
        assert!(extract_kit_colors(&rgba, 4, 4).is_some());

        let short = vec![255u8; 40];
        assert!(extract_kit_colors(&short, SIZE, SIZE).is_none());
        assert!(dominant_colors(&short, SIZE, SIZE, SHIRT).is_empty());
    }

    #[test]
    fn region_edges_are_exclusive() {
        // A color starting exactly on x1 or y1 is outside the region.
        let left_half = Region {
            x0: 0.0,
            y0: 0.0,
            x1: 0.5,
            y1: 1.0,
        };
        let vertical = texture(16, 16, |x, _| {
            if x < 8 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        });
        let clusters = dominant_colors(&vertical, 16, 16, left_half);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].color, [255, 0, 0]);

        let top_half = Region {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 0.5,
        };
        let horizontal = texture(16, 16, |_, y| {
            if y < 8 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        });
        let clusters = dominant_colors(&horizontal, 16, 16, top_half);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].color, [255, 0, 0]);
    }

    #[test]
    fn clusters_merge_within_merge_distance_only() {
        // RGB distance exactly MERGE_DISTANCE does not merge; below it would.
        let whole = Region {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        };
        let rgba = texture(16, 16, |x, _| {
            if x < 8 {
                [0, 0, 0, 255]
            } else {
                [24, 0, 0, 255]
            }
        });
        assert_eq!(dominant_colors(&rgba, 16, 16, whole).len(), 2);
    }

    #[test]
    fn bin_mean_rounds_half_up() {
        // [200,30,30] and [201,31,31] share one 5-bit bin; the .5 mean rounds up.
        let whole = Region {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        };
        let rgba = texture(16, 16, |x, _| {
            if (x / 4) % 2 == 0 {
                [200, 30, 30, 255]
            } else {
                [201, 31, 31, 255]
            }
        });
        let clusters = dominant_colors(&rgba, 16, 16, whole);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].color, [201, 31, 31]);
    }

    #[test]
    fn dominant_colors_returns_the_region_clusters() {
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                [200, 30, 30, 255]
            } else if in_shorts(x, y) {
                [20, 20, 90, 255]
            } else {
                GREY
            }
        });
        let clusters = dominant_colors(&rgba, SIZE, SIZE, SHIRT);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].color, [200, 30, 30]);
        assert!(clusters[0].share > 0.99, "{}", clusters[0].share);
    }

    #[test]
    fn shorts_second_cluster_wins_when_shirt_trim_is_too_small() {
        // Shirt black with a badge under the trim share (about 5%, as logo_is_ignored);
        // shorts black with about 40% of a distinct blue on part of one panel: color 2
        // is the shorts' second cluster.
        let (x0, y0, _, _) = shirt_px();
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                if (x0 + 8..x0 + 28).contains(&x) && (y0 + 8..y0 + 56).contains(&y) {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                }
            } else if in_shorts(x, y) {
                if x < 61 {
                    [20, 20, 90, 255]
                } else {
                    [0, 0, 0, 255]
                }
            } else {
                GREY
            }
        });
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.color1, [0, 0, 0]);
        assert_eq!(kit.color2, [20, 20, 90]);
        assert_eq!(kit.color2_source, Color2Source::ShortsSecond);
    }

    #[test]
    fn one_color_kit_keeps_a_second_shorts_cluster() {
        // Shorts black with about 40% of grey [30,30,30] on part of one panel: a
        // separate cluster (distance about 52 > MERGE_DISTANCE) that is not a distinct
        // color (< DISTINCT_DISTANCE), so color 2 falls back to the shorts' dominant.
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                [0, 0, 0, 255]
            } else if in_shorts(x, y) {
                if x < 61 {
                    [30, 30, 30, 255]
                } else {
                    [0, 0, 0, 255]
                }
            } else {
                GREY
            }
        });
        let kit = extract_kit_colors(&rgba, SIZE, SIZE).expect("colors");
        assert_eq!(kit.color1, [0, 0, 0]);
        assert_eq!(kit.color2, [0, 0, 0]);
        assert_eq!(kit.color2_source, Color2Source::SameAsColor1);
        assert_eq!(kit.shorts.len(), 2);
    }

    #[test]
    fn short_buffer_is_empty_not_partial() {
        let whole = Region {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        };
        // 4 opaque bytes declared 2x2: not 4 * width * height, so empty rather
        // than a partial cluster.
        let rgba = [255, 0, 0, 255];
        assert!(dominant_colors(&rgba, 2, 2, whole).is_empty());
        assert!(extract_kit_colors(&rgba, 2, 2).is_none());
    }

    #[test]
    fn huge_dimensions_are_empty_without_overflow() {
        let whole = Region {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        };
        // The pixel-count arithmetic would overflow: empty, not a panic or a wrap.
        let rgba = vec![255u8; 16];
        assert!(dominant_colors(&rgba, u32::MAX, u32::MAX, whole).is_empty());
        assert!(extract_kit_colors(&rgba, u32::MAX, u32::MAX).is_none());
    }

    /// The community's PES 19 colored template sheet, 128x128 nearest-neighbour, raw RGBA8
    /// (`tests/fixtures/README.md`).
    const TEMPLATE: &[u8] = include_bytes!("../tests/fixtures/kit_col_template_pes19_128.rgba");

    /// The region constants against the real template layout: the sheet paints the shirt's
    /// front and back two reds and the shorts two yellows, so the shirt region must see only
    /// reds and the shorts region only yellows (literals measured on the fixture by
    /// `scripts/provenance/fixtures/color_tools_template_fixture.py`, independently of this
    /// crate). Front and back are near-equal halves, so which red or yellow ranks first is
    /// not asserted; the pair of clusters and their shares are.
    #[test]
    fn template_sheet_regions_land_on_the_shirt_and_shorts() {
        let reds = [[165, 10, 10], [197, 14, 14]];
        let yellows = [[226, 181, 0], [255, 216, 0]];
        let kit = extract_kit_colors(TEMPLATE, 128, 128).expect("colors");

        let shirt: Vec<[u8; 3]> = kit.shirt.iter().take(2).map(|c| c.color).collect();
        assert!(reds.iter().all(|red| shirt.contains(red)), "{shirt:?}");
        for cluster in &kit.shirt[..2] {
            assert!((0.40..=0.55).contains(&cluster.share), "{cluster:?}");
        }
        let shorts: Vec<[u8; 3]> = kit.shorts.iter().take(2).map(|c| c.color).collect();
        assert!(
            yellows.iter().all(|yellow| shorts.contains(yellow)),
            "{shorts:?}"
        );
        for cluster in &kit.shorts[..2] {
            assert!((0.40..=0.55).contains(&cluster.share), "{cluster:?}");
        }

        // The two reds are 32 apart: separate clusters but not distinct, so color 2 is
        // the shorts' dominant yellow.
        assert!(reds.contains(&kit.color1), "{:?}", kit.color1);
        assert!(yellows.contains(&kit.color2), "{:?}", kit.color2);
        assert_eq!(kit.color2_source, Color2Source::Shorts);
    }

    #[test]
    fn deterministic() {
        let rgba = texture(SIZE, SIZE, |x, y| {
            if in_shirt(x, y) {
                [200, 30, 30, 255]
            } else if in_shorts(x, y) {
                [20, 20, 90, 255]
            } else {
                GREY
            }
        });
        assert_eq!(
            extract_kit_colors(&rgba, SIZE, SIZE),
            extract_kit_colors(&rgba, SIZE, SIZE)
        );
    }
}
