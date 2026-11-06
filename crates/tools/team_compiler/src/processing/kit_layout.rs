//! A kit's main texture re-laid out between the two engines' kit UV layouts
//! (`spec/team_compiler/pipeline.md` "4. Per-export non-model steps", Kits, "Layout conversion"):
//! only the two sock islands differ, so the move is a fixed table of rectangles, two bands per
//! sock, each band resampled along u alone; every other texel stays as it was. Also a kit's
//! number atlases re-arranged between the two engines' arrangements of their ten digits (the
//! same section, the glyph atlas sentences): a column on PES 15-17, a row on PES 18-21.

use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

use aesthetics_export::KitLayout;
use dds_convert::{
    BlockCodec, Blocks, ConvertError, Decoded, SourceFormat, decode, encode_dds, resize,
};
use pipeline::MemoryBudget;

use super::texture::decode_charge;

/// The width, in texels, of the texture the table is measured on: a texture of any other size
/// has the table scaled to it.
const TABLE_SIZE: u32 = 2048;

/// The rows both sock islands span, the same in both layouts, in `TABLE_SIZE` units.
const SOCK_ROWS: Range<u32> = 632..1160;

/// One band of a sock island: the columns it spans in each layout, in `TABLE_SIZE` units,
/// half-open, over `SOCK_ROWS`.
struct Band {
    pre_fox: Range<u32>,
    fox: Range<u32>,
}

/// `KIT_LAYOUT_REMAP`: where each band of the two sock islands sits in each layout, read in
/// either direction. Measured on the games' own uniform models; two bands per sock because the
/// pre-Fox island gives the part of the leg near one seam more texels than the rest, which one
/// scale does not follow. The right sock is the left one mirrored (u to 2048 - u).
const KIT_LAYOUT_REMAP: [Band; 4] = [
    // Left sock.
    Band {
        pre_fox: 8..168,
        fox: 8..128,
    },
    Band {
        pre_fox: 168..448,
        fox: 128..376,
    },
    // Right sock.
    Band {
        pre_fox: 1880..2040,
        fox: 1920..2040,
    },
    Band {
        pre_fox: 1600..1880,
        fox: 1672..1920,
    },
];

/// One band's move at one mip level, in that level's texels: the destination rectangle is
/// filled from the source one.
struct Move {
    source: Range<u32>,
    destination: Range<u32>,
    rows: Range<u32>,
}

/// `units`, in `TABLE_SIZE` units, at a level `size` texels across: each end rounded to the
/// nearest texel, a half up.
fn scaled(units: &Range<u32>, size: u32) -> Range<u32> {
    let scale = |unit: u32| rounded_ratio(u64::from(unit) * u64::from(size), u64::from(TABLE_SIZE));
    scale(units.start)..scale(units.end)
}

/// `numerator / denominator` rounded to the nearest whole number, a half up: a length of one
/// of this module's tables scaled to a texture's own size, in texels.
fn rounded_ratio(numerator: u64, denominator: u64) -> u32 {
    u32::try_from((numerator + denominator / 2) / denominator)
        .expect("a table length is at most its table's size, so it scales to at most the texture's")
}

/// The moves at a `width` x `height` level of a texture drawn for `drawn_for`, toward the
/// other layout: a band whose source, destination or rows round to no texel is left out.
fn level_moves(drawn_for: KitLayout, width: u32, height: u32) -> Vec<Move> {
    let rows = scaled(&SOCK_ROWS, height);
    KIT_LAYOUT_REMAP
        .iter()
        .filter_map(|band| {
            let (source, destination) = match drawn_for {
                KitLayout::PreFox => (&band.pre_fox, &band.fox),
                KitLayout::Fox => (&band.fox, &band.pre_fox),
            };
            let source = scaled(source, width);
            let destination = scaled(destination, width);
            let empty = source.is_empty() || destination.is_empty() || rows.is_empty();
            (!empty).then(|| Move {
                source,
                destination,
                rows: rows.clone(),
            })
        })
        .collect()
}

/// `decoded`, a kit's main texture drawn for the `drawn_for` layout, re-laid out for the other
/// one. At every mip level each sock band's destination rectangle is filled with its source
/// rectangle resampled along u, read from the level as it was before any band was written;
/// every other texel keeps its value. A BC1 or BC3 source keeps its blocks wherever no
/// destination rectangle reaches and has the others encoded afresh, so its conversion passes
/// the kit's blocks through as before; any other source comes back as pixels alone, encoded
/// whole by its conversion. Size, mip count, `authored_mips` and the uncompressed record are
/// the source's. The copy
/// returned is the caller's to charge; the re-decode a BC1 or BC3 source's blocks need is
/// charged to `budget` here, while it lives.
///
/// # Errors
///
/// What `dds_convert` returns for a `decoded` it refuses; never for one `decode` produced.
pub(super) fn relaid(
    budget: &Arc<MemoryBudget>,
    decoded: &Decoded,
    drawn_for: KitLayout,
) -> Result<Decoded, ConvertError> {
    let mut mips = Vec::with_capacity(decoded.mips.len());
    let mut moves = Vec::with_capacity(decoded.mips.len());
    for (level, pixels) in decoded.mips.iter().enumerate() {
        let (width, height) = level_size(decoded, level);
        let level_moves = level_moves(drawn_for, width, height);
        mips.push(relaid_level(pixels, width, &level_moves)?);
        moves.push(level_moves);
    }
    let relaid = Decoded {
        width: decoded.width,
        height: decoded.height,
        mips,
        blocks: None,
        authored_mips: decoded.authored_mips,
        uncompressed: decoded.uncompressed,
    };
    let Some(blocks) = &decoded.blocks else {
        return Ok(relaid);
    };
    match blocks.codec {
        BlockCodec::Bc1 | BlockCodec::Bc3 => with_kept_blocks(budget, relaid, blocks, &moves),
        // Every engine reads BC1 and BC3, so their kept blocks reach the game; BC7 blocks
        // reach only PES 19-21 and the rest have no encoder here.
        BlockCodec::Bc2 | BlockCodec::Bc4 | BlockCodec::Bc5 | BlockCodec::Bc7 => Ok(relaid),
    }
}

/// The size of mip `level` of `decoded`: each side halved per level, at least 1.
fn level_size(decoded: &Decoded, level: usize) -> (u32, u32) {
    let halved = |side: u32| side.checked_shr(level as u32).unwrap_or(0).max(1);
    (halved(decoded.width), halved(decoded.height))
}

/// `pixels`, one level `width` texels wide, with `moves` applied.
fn relaid_level(pixels: &[u8], width: u32, moves: &[Move]) -> Result<Vec<u8>, ConvertError> {
    let mut relaid = pixels.to_vec();
    for step in moves {
        let band = rectangle(pixels, width, &step.source, &step.rows);
        let band_height = span(&step.rows);
        let resampled = resize(
            &band,
            span(&step.source),
            band_height,
            span(&step.destination),
            band_height,
        )?;
        let row_len = span(&step.destination) as usize * 4;
        for (row, y) in resampled.chunks_exact(row_len).zip(step.rows.clone()) {
            let start = texel_offset(width, step.destination.start, y);
            relaid[start..start + row_len].copy_from_slice(row);
        }
    }
    Ok(relaid)
}

/// The texels of `columns` x `rows` of a level `width` texels wide, row by row.
fn rectangle(pixels: &[u8], width: u32, columns: &Range<u32>, rows: &Range<u32>) -> Vec<u8> {
    let row_len = span(columns) as usize * 4;
    let mut texels = Vec::with_capacity(row_len * span(rows) as usize);
    for y in rows.clone() {
        let start = texel_offset(width, columns.start, y);
        texels.extend_from_slice(&pixels[start..start + row_len]);
    }
    texels
}

fn span(range: &Range<u32>) -> u32 {
    range.end - range.start
}

/// The byte offset of texel (`x`, `y`) in an RGBA8 level `width` texels wide.
fn texel_offset(width: u32, x: u32, y: u32) -> usize {
    (y as usize * width as usize + x as usize) * 4
}

/// `relaid` given blocks in `source`'s codec: per level, every 4x4 block a destination
/// rectangle of `moves` reaches encoded afresh from the re-laid pixels, every other block
/// `source`'s own, and the pixels made what those blocks decode to. The fresh decode is
/// charged to `budget` until it is freed, on return.
fn with_kept_blocks(
    budget: &Arc<MemoryBudget>,
    mut relaid: Decoded,
    source: &Blocks,
    moves: &[Vec<Move>],
) -> Result<Decoded, ConvertError> {
    let encoded = encode_dds(&relaid, source.codec)?;
    let _fresh_charge = decode_charge(budget, &encoded, SourceFormat::Dds)?;
    let fresh = decode(&encoded, SourceFormat::Dds)?;
    let fresh_blocks = fresh
        .blocks
        .expect("a BC1 or BC3 DDS decodes with its blocks");
    let mut kept = source.mips.clone();
    for (level, level_moves) in moves.iter().enumerate() {
        let (width, height) = level_size(&relaid, level);
        let columns = width.div_ceil(4);
        let block_len = kept[level].len() / (columns * height.div_ceil(4)) as usize;
        for (column, row) in reached_blocks(level_moves, width, height) {
            let block = (row * columns + column) as usize * block_len;
            kept[level][block..block + block_len]
                .copy_from_slice(&fresh_blocks.mips[level][block..block + block_len]);
            // The block's texels, clipped to the level, as the fresh block decodes them;
            // the pixels of a kept block already are what it decodes to.
            let x = column * 4..(column * 4 + 4).min(width);
            for y in row * 4..(row * 4 + 4).min(height) {
                let start = texel_offset(width, x.start, y);
                let end = start + span(&x) as usize * 4;
                relaid.mips[level][start..end].copy_from_slice(&fresh.mips[level][start..end]);
            }
        }
    }
    relaid.blocks = Some(Blocks {
        codec: source.codec,
        mips: kept,
    });
    Ok(relaid)
}

/// The (column, row) of every 4x4 block of a `width` x `height` level that a destination
/// rectangle of `moves` overlaps.
fn reached_blocks(moves: &[Move], width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut reached = Vec::new();
    for row in 0..height.div_ceil(4) {
        for column in 0..width.div_ceil(4) {
            let overlaps = moves.iter().any(|step| {
                column * 4 < step.destination.end
                    && column * 4 + 4 > step.destination.start
                    && row * 4 < step.rows.end
                    && row * 4 + 4 > step.rows.start
            });
            if overlaps {
                reached.push((column, row));
            }
        }
    }
    reached
}

/// The length, in texels, of the atlas `NUMBER_ATLAS_SLOTS` is measured in: an atlas of any
/// other length has the table scaled to it.
const NUMBER_ATLAS_LENGTH: u32 = 2048;

/// Where a PES 18-21 number atlas, a row, keeps each digit: digit d's slot, its start and end
/// in texels along a `NUMBER_ATLAS_LENGTH`-long row, over the row's whole height. Measured on
/// the games' stock atlases (`scripts/provenance/kit_uv/number_atlas/`). Each slot is 160
/// long except digit 1's, which ends where digit 2's starts. A PES 15-17 number atlas, a
/// column, keeps digit d in the equal tenth `[204.8 d, 204.8 (d + 1))` of its length, over its
/// whole width; a slot is such a cell scaled by 1.25.
const NUMBER_ATLAS_SLOTS: [(u32, u32); 10] = [
    (0, 160),
    (190, 340),
    (340, 500),
    (540, 700),
    (740, 900),
    (940, 1100),
    (1140, 1300),
    (1340, 1500),
    (1540, 1700),
    (1740, 1900),
];

/// The arrangement of a `width` x `height` number atlas, read from its shape: taller than wide
/// is a column, the PES 15-17 arrangement (`KitLayout::PreFox`); wider than tall a row, the
/// PES 18-21 one (`KitLayout::Fox`); a square is neither.
pub(super) fn atlas_arrangement(width: u32, height: u32) -> Option<KitLayout> {
    match width.cmp(&height) {
        Ordering::Less => Some(KitLayout::PreFox),
        Ordering::Greater => Some(KitLayout::Fox),
        Ordering::Equal => None,
    }
}

/// One digit's move between the two arrangements, in texels of the top levels: the source
/// rectangle `columns` x `rows` is resized to `glyph_width` x `glyph_height`, and the first
/// `kept_columns` columns of every row of the glyph are written from (`x`, `y`) on.
struct GlyphMove {
    columns: Range<u32>,
    rows: Range<u32>,
    glyph_width: u32,
    glyph_height: u32,
    x: u32,
    y: u32,
    kept_columns: u32,
}

/// `decoded`, a `kit_back`, `kit_chest` or `kit_leg` number atlas, re-arranged into `to`'s
/// arrangement; `None` when its shape is already `to`'s or a square. Every digit moves by a
/// uniform scale, top edges together: a column's cell, scaled by 1.25, fills its slot of a row
/// as long as the column and an eighth of that high, the part past the slot's end left out; a
/// row's slot, scaled by 0.8, goes to the top-left of its cell of a column as long as the row
/// and a sixteenth of that wide. The table's edges are scaled to the atlas's length and
/// rounded to the nearest texel. Only the top level is re-arranged: the result has one level
/// and `authored_mips` false, so its conversion generates the chain, and no blocks.
///
/// # Errors
///
/// What `dds_convert::resize` returns for a glyph it refuses; never for a `decoded` that
/// `decode` produced, since every glyph resized has a texel.
pub(super) fn number_atlas_rearranged(
    decoded: &Decoded,
    to: KitLayout,
) -> Result<Option<Decoded>, ConvertError> {
    if atlas_arrangement(decoded.width, decoded.height).is_none_or(|drawn_as| drawn_as == to) {
        return Ok(None);
    }
    let top = decoded
        .mips
        .first()
        .expect("a decoded texture has its top level");
    let length = decoded.width.max(decoded.height);
    let (width, height) = match to {
        KitLayout::Fox => (length, (length / 8).max(1)),
        KitLayout::PreFox => ((length / 16).max(1), length),
    };
    let scaled_edge = |unit: u32| {
        rounded_ratio(
            u64::from(unit) * u64::from(length),
            u64::from(NUMBER_ATLAS_LENGTH),
        )
    };
    // Outside its glyphs every stock atlas holds the glyph's color at alpha 0, its top-left
    // texel included, so filtering at a glyph's edge does not darken it; a fixed color would.
    let mut pixels = top[..4].repeat(width as usize * height as usize);
    for (digit, &(start, end)) in (0u64..).zip(&NUMBER_ATLAS_SLOTS) {
        // 204.8 d of a 2048-long column is d tenths of any length.
        let cell_edge = |d: u64| rounded_ratio(d * u64::from(length), 10);
        let cell = cell_edge(digit)..cell_edge(digit + 1);
        let slot = scaled_edge(start)..scaled_edge(end);
        let step = match to {
            KitLayout::Fox => GlyphMove {
                columns: 0..decoded.width,
                rows: cell,
                // The column's width, 128 of a 2048-long atlas, scaled by 1.25.
                glyph_width: scaled_edge(160),
                glyph_height: height,
                x: slot.start,
                y: 0,
                kept_columns: span(&slot),
            },
            KitLayout::PreFox => GlyphMove {
                columns: slot,
                rows: 0..decoded.height,
                // The slot's length scaled by 0.8.
                glyph_width: rounded_ratio(
                    4 * u64::from(end - start) * u64::from(length),
                    5 * u64::from(NUMBER_ATLAS_LENGTH),
                ),
                glyph_height: span(&cell),
                x: 0,
                y: cell.start,
                kept_columns: width,
            },
        };
        write_glyph(&mut pixels, width, top, decoded.width, &step)?;
    }
    Ok(Some(Decoded {
        width,
        height,
        mips: vec![pixels],
        blocks: None,
        authored_mips: false,
        uncompressed: decoded.uncompressed,
    }))
}

/// `step` applied from `source`, a level `source_width` texels wide, onto `pixels`, a level
/// `width` texels wide. A digit whose source, glyph or kept part rounds to no texel (only on
/// an atlas a few texels long) is left out.
fn write_glyph(
    pixels: &mut [u8],
    width: u32,
    source: &[u8],
    source_width: u32,
    step: &GlyphMove,
) -> Result<(), ConvertError> {
    // Rounding can make a slot one texel longer than its glyph, or a glyph one texel wider
    // than a column: a glyph is never stretched to fill its slot, nor written past the column.
    let kept_columns = step.kept_columns.min(step.glyph_width);
    if step.columns.is_empty()
        || step.rows.is_empty()
        || kept_columns == 0
        || step.glyph_height == 0
    {
        return Ok(());
    }
    let glyph = resize(
        &rectangle(source, source_width, &step.columns, &step.rows),
        span(&step.columns),
        span(&step.rows),
        step.glyph_width,
        step.glyph_height,
    )?;
    let glyph_row_len = step.glyph_width as usize * 4;
    let kept_len = kept_columns as usize * 4;
    for (row, y) in glyph.chunks_exact(glyph_row_len).zip(step.y..) {
        let start = texel_offset(width, step.x, y);
        pixels[start..start + kept_len].copy_from_slice(&row[..kept_len]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A budget no test fills: these tests check pixels, not charges.
    fn unlimited() -> Arc<MemoryBudget> {
        MemoryBudget::new(usize::MAX)
    }

    /// The rows of both islands on a 256-texel texture: the table divided by 8.
    const ROWS_256: Range<u32> = 79..145;
    /// The four bands' columns on a 256-texel texture, in `KIT_LAYOUT_REMAP`'s order: left
    /// band 1, left band 2, right band 1, right band 2.
    const PRE_FOX_256: [Range<u32>; 4] = [1..21, 21..56, 235..255, 200..235];
    const FOX_256: [Range<u32>; 4] = [1..16, 16..47, 240..255, 209..240];

    /// A one-level `width` x `height` texture with no blocks, as a raster decodes, whose
    /// texel at (x, y) is `color(x, y)`.
    fn texture(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Decoded {
        Decoded {
            width,
            height,
            mips: vec![pixels(width, height, color)],
            blocks: None,
            authored_mips: false,
            uncompressed: false,
        }
    }

    fn pixels(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&color(x, y));
            }
        }
        pixels
    }

    /// A color different in every texel of a 256 x 256 texture.
    fn distinct(x: u32, y: u32) -> [u8; 4] {
        [x as u8, y as u8, (x * 7 + y * 13) as u8, 255]
    }

    fn texel(mip: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let at = texel_offset(width, x, y);
        [mip[at], mip[at + 1], mip[at + 2], mip[at + 3]]
    }

    /// Whether (x, y) of a 256 x 256 texture lies in one of `rectangles` (over `ROWS_256`).
    fn inside(rectangles: &[Range<u32>; 4], x: u32, y: u32) -> bool {
        ROWS_256.contains(&y) && rectangles.iter().any(|columns| columns.contains(&x))
    }

    /// The destination rectangles of a texture drawn for `drawn_for`.
    fn destinations(drawn_for: KitLayout) -> &'static [Range<u32>; 4] {
        match drawn_for {
            KitLayout::PreFox => &FOX_256,
            KitLayout::Fox => &PRE_FOX_256,
        }
    }

    /// The source rectangles of a texture drawn for `drawn_for`.
    fn sources(drawn_for: KitLayout) -> &'static [Range<u32>; 4] {
        match drawn_for {
            KitLayout::PreFox => &PRE_FOX_256,
            KitLayout::Fox => &FOX_256,
        }
    }

    /// One flat color per band, in `KIT_LAYOUT_REMAP`'s order.
    const BAND_COLORS: [[u8; 4]; 4] = [
        [200, 30, 30, 255],
        [30, 200, 30, 255],
        [30, 30, 200, 255],
        [220, 220, 40, 255],
    ];

    /// A 256 x 256 texture whose four `rectangles` (over `ROWS_256`) are each one flat color
    /// of `BAND_COLORS`, and whose every other texel is `distinct`.
    fn banded(rectangles: &[Range<u32>; 4]) -> Decoded {
        texture(256, 256, |x, y| {
            let band = (0..4).find(|&band| ROWS_256.contains(&y) && rectangles[band].contains(&x));
            match band {
                Some(band) => BAND_COLORS[band],
                None => distinct(x, y),
            }
        })
    }

    #[test]
    fn flat_bands_go_to_fox_flat_and_come_back_exactly() {
        let pre_fox = banded(&PRE_FOX_256);

        let fox = relaid(&unlimited(), &pre_fox, KitLayout::PreFox).unwrap();
        for (band, columns) in FOX_256.iter().enumerate() {
            for y in ROWS_256 {
                for x in columns.clone() {
                    assert_eq!(
                        texel(&fox.mips[0], 256, x, y),
                        BAND_COLORS[band],
                        "({x}, {y})"
                    );
                }
            }
        }
        assert_eq!(relaid(&unlimited(), &fox, KitLayout::Fox).unwrap(), pre_fox);
    }

    #[test]
    fn no_texel_outside_the_destination_rectangles_moves() {
        let source = texture(256, 256, distinct);
        for drawn_for in [KitLayout::PreFox, KitLayout::Fox] {
            let relaid = relaid(&unlimited(), &source, drawn_for).unwrap();
            let mut moved = 0;
            for y in 0..256 {
                for x in 0..256 {
                    let (before, after) = (
                        texel(&source.mips[0], 256, x, y),
                        texel(&relaid.mips[0], 256, x, y),
                    );
                    if inside(destinations(drawn_for), x, y) {
                        moved += usize::from(before != after);
                    } else {
                        assert_eq!(after, before, "{drawn_for:?} ({x}, {y})");
                    }
                }
            }
            // The destination rectangles did change: the move happened.
            assert!(moved > 0, "{drawn_for:?}");
        }
    }

    /// `decoded` with every row reversed: the texture mirrored in u.
    fn mirrored(decoded: &Decoded) -> Decoded {
        let mips = decoded
            .mips
            .iter()
            .enumerate()
            .map(|(level, mip)| {
                let (width, _) = level_size(decoded, level);
                mip.chunks_exact(width as usize * 4)
                    .flat_map(|row| row.as_chunks::<4>().0.iter().rev().flatten().copied())
                    .collect()
            })
            .collect();
        Decoded {
            mips,
            ..decoded.clone()
        }
    }

    // The bands are flat because the resampler computes in `f32`: a varied band and its mirror
    // image can round one value differently (one channel of one texel, on 256 x 256 scattered
    // values). A flat band resamples exactly, and a right-sock rectangle that is not the
    // mirror of its left partner still writes the wrong texels.
    #[test]
    fn the_right_sock_moves_as_the_left_one_mirrored() {
        for drawn_for in [KitLayout::PreFox, KitLayout::Fox] {
            let source = banded(sources(drawn_for));
            assert_eq!(
                relaid(&unlimited(), &mirrored(&source), drawn_for).unwrap(),
                mirrored(&relaid(&unlimited(), &source, drawn_for).unwrap()),
                "{drawn_for:?}"
            );
        }
    }

    #[test]
    fn each_level_is_re_laid_at_its_own_scale() {
        let top = texture(256, 256, distinct);
        let lower = texture(128, 128, |x, y| distinct(255 - x, y));
        let source = Decoded {
            mips: vec![top.mips[0].clone(), lower.mips[0].clone()],
            authored_mips: true,
            ..top.clone()
        };

        let relaid_source = relaid(&unlimited(), &source, KitLayout::PreFox).unwrap();

        assert_eq!(relaid_source.mips.len(), 2);
        assert_eq!(
            relaid_source.mips[0],
            relaid(&unlimited(), &top, KitLayout::PreFox).unwrap().mips[0]
        );
        assert_eq!(
            relaid_source.mips[1],
            relaid(&unlimited(), &lower, KitLayout::PreFox)
                .unwrap()
                .mips[0]
        );
        assert_ne!(relaid_source.mips[1], source.mips[1]);
    }

    #[test]
    fn a_level_too_small_for_a_band_leaves_that_band_s_texels_alone() {
        // At 8 texels the island's rows are 2 to 5. Fox to pre-Fox, left band 1 is column 0
        // onto itself; left band 2's source (128 to 376) and right band 1's (1920 to 2040)
        // round to no column, so their destinations, columns 1 and 7, keep their texels; right
        // band 2 moves column 7 onto column 6. At 2 texels the rows round to none and nothing
        // moves.
        let distinct_8 = |x: u32, y: u32| [x as u8 * 30, y as u8 * 30, 100, 255];
        let source = Decoded {
            mips: vec![
                pixels(8, 8, distinct_8),
                pixels(4, 4, |x, y| distinct_8(x + 2, y + 2)),
                pixels(2, 2, |x, y| distinct_8(x + 5, y + 5)),
            ],
            authored_mips: true,
            ..texture(8, 8, distinct_8)
        };

        let relaid = relaid(&unlimited(), &source, KitLayout::Fox).unwrap();

        let expected = pixels(8, 8, |x, y| {
            if x == 6 && (2..5).contains(&y) {
                distinct_8(7, y)
            } else {
                distinct_8(x, y)
            }
        });
        assert_eq!(relaid.mips[0], expected);
        assert_eq!(relaid.mips[2], source.mips[2]);
    }

    /// A one-level 256 x 256 BC1 texture whose blocks are scattered bytes, as `decode` reads
    /// it. An encoder rarely gives such blocks back from their own decode, so a block
    /// encoded afresh shows; the blocks of an encoded image often come back byte for byte.
    fn scattered_bc1() -> Decoded {
        let blocks = (0..64 * 64 * 8u32)
            .map(|i| (i.wrapping_mul(0x9E37_79B1) >> 13) as u8)
            .collect();
        let carrier = Decoded {
            blocks: Some(Blocks {
                codec: BlockCodec::Bc1,
                mips: vec![blocks],
            }),
            authored_mips: true,
            ..texture(256, 256, distinct)
        };
        decode(
            &encode_dds(&carrier, BlockCodec::Bc1).unwrap(),
            SourceFormat::Dds,
        )
        .unwrap()
    }

    #[test]
    fn a_bc1_source_keeps_every_block_no_destination_reaches() {
        let source = scattered_bc1();
        let source_blocks = source.blocks.as_ref().unwrap();

        let relaid = relaid(&unlimited(), &source, KitLayout::PreFox).unwrap();

        let blocks = relaid.blocks.as_ref().expect("BC1 blocks kept");
        assert_eq!(blocks.codec, BlockCodec::Bc1);
        assert_eq!(blocks.mips.len(), source_blocks.mips.len());
        // Level 0, 64 x 64 blocks of 8 bytes: one a Fox rectangle overlaps is fresh, every
        // other one the source's.
        let mut fresh = 0;
        for row in 0..64u32 {
            for column in 0..64u32 {
                let at = (row * 64 + column) as usize * 8;
                let block = &blocks.mips[0][at..at + 8];
                let reached = (column * 4..column * 4 + 4)
                    .any(|x| (row * 4..row * 4 + 4).any(|y| inside(&FOX_256, x, y)));
                if reached {
                    fresh += usize::from(block != &source_blocks.mips[0][at..at + 8]);
                } else {
                    assert_eq!(
                        block,
                        &source_blocks.mips[0][at..at + 8],
                        "({column}, {row})"
                    );
                }
            }
        }
        assert!(fresh > 0);
        // The pixels are what the blocks decode to.
        let redecoded = decode(
            &encode_dds(&relaid, BlockCodec::Bc1).unwrap(),
            SourceFormat::Dds,
        )
        .unwrap();
        assert_eq!(redecoded.mips, relaid.mips);
    }

    // On a 2048 or 1024 texture the rectangles' edges sit on block edges, which no 256-texel
    // test shows: a block that only touches a rectangle is not reached, so it is kept.
    #[test]
    fn a_block_that_only_touches_a_destination_rectangle_is_not_reached() {
        let step = Move {
            source: 0..4,
            destination: 8..16,
            rows: 12..20,
        };
        assert_eq!(
            reached_blocks(&[step], 32, 32),
            [(2, 3), (3, 3), (2, 4), (3, 4)]
        );
    }

    #[test]
    fn a_raster_or_bc7_source_comes_back_without_blocks() {
        let raster = texture(256, 256, distinct);
        assert_eq!(
            relaid(&unlimited(), &raster, KitLayout::PreFox)
                .unwrap()
                .blocks,
            None
        );
        let bc7 = decode(
            &encode_dds(&raster, BlockCodec::Bc7).unwrap(),
            SourceFormat::Dds,
        )
        .unwrap();
        assert_eq!(
            relaid(&unlimited(), &bc7, KitLayout::PreFox)
                .unwrap()
                .blocks,
            None
        );
    }

    /// One opaque color per digit.
    const DIGIT_COLORS: [[u8; 4]; 10] = [
        [200, 30, 30, 255],
        [30, 200, 30, 255],
        [30, 30, 200, 255],
        [220, 220, 40, 255],
        [40, 220, 220, 255],
        [220, 40, 220, 255],
        [120, 60, 10, 255],
        [10, 120, 60, 255],
        [60, 10, 120, 255],
        [250, 250, 250, 255],
    ];

    /// The texels of a row between its digits' slots: a color no digit has, at alpha 0.
    const GAP: [u8; 4] = [90, 160, 110, 0];

    /// Where each digit's slot lies along a 2048-long PES 18-21 row, written out apart from
    /// `NUMBER_ATLAS_SLOTS` so a wrong table entry shows.
    const SLOTS_2048: [Range<u32>; 10] = [
        0..160,
        190..340,
        340..500,
        540..700,
        740..900,
        940..1100,
        1140..1300,
        1340..1500,
        1540..1700,
        1740..1900,
    ];

    /// Where each digit's cell starts along a PES 15-17 column 2048 or 1024 long, its tenth
    /// rounded to the nearest texel, then the column's end.
    const CELL_EDGES_2048: [u32; 11] = [0, 205, 410, 614, 819, 1024, 1229, 1434, 1638, 1843, 2048];
    const CELL_EDGES_1024: [u32; 11] = [0, 102, 205, 307, 410, 512, 614, 717, 819, 922, 1024];

    /// The digit whose cell, between `edges`, holds `y`.
    fn cell_digit(edges: &[u32; 11], y: u32) -> usize {
        edges
            .windows(2)
            .position(|cell| (cell[0]..cell[1]).contains(&y))
            .expect("y lies inside the column")
    }

    /// A one-level column `width` wide whose cells, between `edges`, are each flat in their
    /// digit's color.
    fn column_atlas(width: u32, edges: &[u32; 11]) -> Decoded {
        texture(width, edges[10], |_, y| DIGIT_COLORS[cell_digit(edges, y)])
    }

    /// A one-level row `length` x `height`, `length` 2048 or 1024, whose slots (`SLOTS_2048`
    /// scaled to `length`) are each flat in their digit's color and whose other texels are
    /// `GAP`. Slot 0 starts at 0, so the top-left texel is digit 0's color.
    fn row_atlas(length: u32, height: u32) -> Decoded {
        texture(length, height, |x, _| {
            let x_2048 = x * 2048 / length;
            match SLOTS_2048.iter().position(|slot| slot.contains(&x_2048)) {
                Some(digit) => DIGIT_COLORS[digit],
                None => GAP,
            }
        })
    }

    #[test]
    fn a_column_becomes_a_row_with_each_digit_filling_its_slot() {
        let row = number_atlas_rearranged(&column_atlas(128, &CELL_EDGES_2048), KitLayout::Fox)
            .unwrap()
            .expect("a column re-arranged for a Fox target");

        assert_eq!((row.width, row.height), (2048, 256));
        assert_eq!(
            (row.mips.len(), row.authored_mips, &row.blocks),
            (1, false, &None)
        );
        let top = &row.mips[0];
        for (digit, slot) in SLOTS_2048.iter().enumerate() {
            let middle = (slot.start + slot.end) / 2;
            for x in [slot.start, middle, slot.end - 1] {
                for y in [0, 128, 255] {
                    assert_eq!(
                        texel(top, 2048, x, y),
                        DIGIT_COLORS[digit],
                        "digit {digit} at ({x}, {y})"
                    );
                }
            }
        }
        // Between slots 0 and 1, and past the 9: the column's top-left texel.
        for x in [175, 1975] {
            assert_eq!(texel(top, 2048, x, 128), DIGIT_COLORS[0], "x {x}");
        }
    }

    #[test]
    fn a_row_becomes_a_column_with_each_digit_scaled_into_its_cell() {
        let column = number_atlas_rearranged(&row_atlas(2048, 256), KitLayout::PreFox)
            .unwrap()
            .expect("a row re-arranged for a pre-Fox target");

        assert_eq!((column.width, column.height), (128, 2048));
        assert_eq!(
            (column.mips.len(), column.authored_mips, &column.blocks),
            (1, false, &None)
        );
        let top = &column.mips[0];
        for digit in 0..10 {
            let middle = (CELL_EDGES_2048[digit] + CELL_EDGES_2048[digit + 1]) / 2;
            assert_eq!(
                texel(top, 128, 60, middle),
                DIGIT_COLORS[digit],
                "digit {digit}"
            );
        }
        // Digit 1's slot is 150 long, so its glyph is 120 wide (0.8 x 150) and the rest of its
        // cell is the row's top-left texel; none of it is digit 2's color, which starts where
        // slot 1 ends.
        for y in CELL_EDGES_2048[1]..CELL_EDGES_2048[2] {
            for x in 0..128 {
                let expected = if x < 120 {
                    DIGIT_COLORS[1]
                } else {
                    DIGIT_COLORS[0]
                };
                assert_eq!(texel(top, 128, x, y), expected, "({x}, {y})");
            }
        }
    }

    #[test]
    fn a_1024_long_atlas_has_the_table_at_half_scale() {
        let row = number_atlas_rearranged(&column_atlas(64, &CELL_EDGES_1024), KitLayout::Fox)
            .unwrap()
            .expect("a column re-arranged for a Fox target");
        assert_eq!((row.width, row.height), (1024, 128));
        for (digit, slot) in SLOTS_2048.iter().enumerate() {
            let middle = (slot.start + slot.end) / 4;
            assert_eq!(
                texel(&row.mips[0], 1024, middle, 64),
                DIGIT_COLORS[digit],
                "digit {digit}"
            );
        }

        let column = number_atlas_rearranged(&row_atlas(1024, 128), KitLayout::PreFox)
            .unwrap()
            .expect("a row re-arranged for a pre-Fox target");
        assert_eq!((column.width, column.height), (64, 1024));
        for digit in 0..10 {
            let middle = (CELL_EDGES_1024[digit] + CELL_EDGES_1024[digit + 1]) / 2;
            assert_eq!(
                texel(&column.mips[0], 64, 30, middle),
                DIGIT_COLORS[digit],
                "digit {digit}"
            );
        }
    }

    #[test]
    fn an_atlas_in_the_target_s_arrangement_or_square_is_left_alone() {
        let column = column_atlas(128, &CELL_EDGES_2048);
        assert_eq!(
            number_atlas_rearranged(&column, KitLayout::PreFox).unwrap(),
            None
        );
        let row = row_atlas(2048, 256);
        assert_eq!(number_atlas_rearranged(&row, KitLayout::Fox).unwrap(), None);
        let square = texture(256, 256, distinct);
        for to in [KitLayout::PreFox, KitLayout::Fox] {
            assert_eq!(
                number_atlas_rearranged(&square, to).unwrap(),
                None,
                "{to:?}"
            );
        }
    }

    // At 8 texels long, digit 2's tenth (rows 1.6 to 2.4) rounds to no row, so its slot,
    // texel 1, keeps the top-left texel; digit 3's cell, row 2, still fills its slot, texel 2.
    #[test]
    fn a_digit_that_rounds_to_no_texel_on_a_tiny_atlas_is_left_out() {
        let column = texture(4, 8, |_, y| DIGIT_COLORS[y as usize]);

        let row = number_atlas_rearranged(&column, KitLayout::Fox)
            .unwrap()
            .expect("a column re-arranged for a Fox target");

        assert_eq!((row.width, row.height), (8, 1));
        assert_eq!(texel(&row.mips[0], 8, 1, 0), DIGIT_COLORS[0]);
        assert_eq!(texel(&row.mips[0], 8, 2, 0), DIGIT_COLORS[2]);
    }

    // At 6 texels long, below the deep pass's smallest atlas but a texture `decode` reads, a
    // glyph is 160 * 6 / 2048 = 0.47 texels wide and rounds to none: every digit is left out,
    // not resized to nothing, which `resize` refuses.
    #[test]
    fn a_glyph_that_rounds_to_no_texel_is_left_out_rather_than_resized() {
        let column = texture(2, 6, |_, y| DIGIT_COLORS[y as usize]);

        let row = number_atlas_rearranged(&column, KitLayout::Fox)
            .unwrap()
            .expect("a column re-arranged for a Fox target");

        assert_eq!((row.width, row.height), (6, 1));
        for x in 0..6 {
            assert_eq!(texel(&row.mips[0], 6, x, 0), DIGIT_COLORS[0], "texel {x}");
        }
    }
}
