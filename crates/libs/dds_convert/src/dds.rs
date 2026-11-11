//! DDS and FTEX sources -> [`Decoded`]: container walk via `ftex::dds`,
//! block decode via `block_compression`, mask decode for uncompressed
//! layouts; and the passthrough that rewraps kept blocks without a decode.

use block_compression::CompressionVariant;
use block_compression::decode::decompress_blocks_as_rgba8;
use ftex::PixelFormat;
use ftex::dds::{DdsLayout, DdsPixel, read_layout};

use crate::{BlockCodec, Blocks, ConvertError, Decoded, Target, alloc_len, encode};

/// Decodes an FTEX source by converting it to DDS first; the DDS path then
/// applies unchanged.
pub(crate) fn decode_ftex(bytes: &[u8]) -> Result<Decoded, ConvertError> {
    decode_dds(&ftex::ftex_to_dds(bytes)?)
}

/// Decodes a DDS source (already unwrapped by the caller): every mip the
/// file carries, plus the raw blocks when the codec is a block format a
/// target might keep.
pub(crate) fn decode_dds(dds: &[u8]) -> Result<Decoded, ConvertError> {
    let layout = read_layout(dds)?;
    let codec = block_codec(layout.pixel);
    // block_compression offsets its RGBA output in u32 (decode.rs:221): a
    // padded level-0 mip past 4 GiB would wrap the offset.
    if codec.is_some() {
        let padded_rgba = u64::from(layout.width)
            .div_ceil(4)
            .saturating_mul(4)
            .saturating_mul(u64::from(layout.height).div_ceil(4))
            .saturating_mul(4)
            .saturating_mul(4);
        if padded_rgba > u64::from(u32::MAX) {
            return Err(ConvertError::Unsupported(
                "block texture past the decoder's 4 GiB limit",
            ));
        }
    }
    let data = mip_data(dds, &layout)?;
    let mut mips = Vec::with_capacity(data.len());
    for (level, data) in (0..).zip(&data) {
        let width = (layout.width >> level).max(1);
        let height = (layout.height >> level).max(1);
        mips.push(decode_mip(&layout, codec, level, width, height, data)?);
    }

    let blocks = codec.map(|(codec, _)| Blocks {
        codec,
        mips: data.iter().map(|mip| mip.to_vec()).collect(),
    });
    Ok(Decoded {
        width: layout.width,
        height: layout.height,
        mips,
        blocks,
        authored_mips: true,
        // A block codec is the only `DdsPixel` `block_codec` names: none
        // means the mips decoded through channel masks, stored uncompressed.
        uncompressed: codec.is_none(),
    })
}

/// The finished container for a DDS source (already unwrapped by the
/// caller) whose blocks `target` keeps as they are, built from the source's
/// own block data without decoding a pixel; `None` when the source has to
/// be decoded and encoded instead.
pub(crate) fn pass_through(dds: &[u8], target: Target) -> Result<Option<Vec<u8>>, ConvertError> {
    let layout = read_layout(dds)?;
    let Some((codec, _)) = block_codec(layout.pixel) else {
        return Ok(None);
    };
    if !encode::compatible(target.version, codec, target.role) {
        return Ok(None);
    }
    let blocks = mip_data(dds, &layout)?;
    encode::container(
        target.version,
        encode::pixel_format(codec),
        layout.width,
        layout.height,
        &blocks,
    )
    .map(Some)
}

/// Each mip level's bytes in the DDS stream, top level first.
fn mip_data<'a>(dds: &'a [u8], layout: &DdsLayout) -> Result<Vec<&'a [u8]>, ConvertError> {
    let mut data = Vec::with_capacity(layout.mipmaps as usize);
    let mut offset = layout.data_offset;
    for level in 0..layout.mipmaps {
        let width = (layout.width >> level).max(1);
        let height = (layout.height >> level).max(1);
        let size = mip_size(layout, level, width, height)?;
        let end = usize::try_from(size)
            .ok()
            .and_then(|size| offset.checked_add(size))
            .ok_or(ConvertError::Truncated)?;
        data.push(dds.get(offset..end).ok_or(ConvertError::Truncated)?);
        offset = end;
    }
    Ok(data)
}

/// The row pitch a level's data uses in the stream: level 0 keeps the
/// header's declared pitch, lower levels round their tight width up to a
/// multiple of 4 bytes (the DWORD rule those exporters follow).
fn mip_row_pitch(layout: &DdsLayout, level: u32, tight_row: u64) -> u64 {
    match layout.row_pitch {
        Some(pitch) if level == 0 => u64::from(pitch),
        Some(_) => tight_row.div_ceil(4) * 4,
        None => tight_row,
    }
}

/// Bytes one mip level occupies in the DDS stream.
fn mip_size(layout: &DdsLayout, level: u32, width: u32, height: u32) -> Result<u64, ConvertError> {
    if let Some(row) = layout.pixel.row_bytes(width) {
        return mip_row_pitch(layout, level, row)
            .checked_mul(u64::from(height))
            .ok_or(ConvertError::Truncated);
    }
    // `row_bytes` is None exactly for the block-compressed formats.
    let DdsPixel::Format(format) = layout.pixel else {
        unreachable!("row_bytes is Some for every Uncompressed layout")
    };
    Ok(ftex::mip_size(format, width, height, 1, 0) as u64)
}

/// The block codec behind `pixel`, when it is one this crate keeps or
/// emits, paired with its `block_compression` variant. `None` for a
/// pixel-stored layout, the notion `Decoded::uncompressed` records.
pub(crate) fn block_codec(pixel: DdsPixel) -> Option<(BlockCodec, CompressionVariant)> {
    let format = match pixel {
        DdsPixel::Format(format) => format,
        DdsPixel::Uncompressed { .. } => return None,
    };
    block_codec_of(format)
}

/// The block codec `format` is, when it is one this crate keeps or emits,
/// paired with its `block_compression` variant: `None` for a pixel-stored
/// format and for a block format no target keeps.
pub(crate) fn block_codec_of(format: PixelFormat) -> Option<(BlockCodec, CompressionVariant)> {
    Some(match format {
        PixelFormat::Bc1 => (BlockCodec::Bc1, CompressionVariant::BC1),
        PixelFormat::Bc2 => (BlockCodec::Bc2, CompressionVariant::BC2),
        PixelFormat::Bc3 => (BlockCodec::Bc3, CompressionVariant::BC3),
        PixelFormat::Bc4 => (BlockCodec::Bc4, CompressionVariant::BC4),
        PixelFormat::Bc5 => (BlockCodec::Bc5, CompressionVariant::BC5),
        PixelFormat::Bc7 => (
            BlockCodec::Bc7,
            CompressionVariant::BC7(block_compression::BC7Settings::alpha_basic()),
        ),
        _ => return None,
    })
}

/// One mip level to straight-alpha RGBA8.
fn decode_mip(
    layout: &DdsLayout,
    codec: Option<(BlockCodec, CompressionVariant)>,
    level: u32,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<Vec<u8>, ConvertError> {
    // The stream row pitch for the row-stored layouts; the block arm does
    // not read it.
    let row_pitch = match layout.pixel.row_bytes(width) {
        Some(tight) => usize::try_from(mip_row_pitch(layout, level, tight))
            .map_err(|_| ConvertError::Truncated)?,
        None => 0,
    };
    match layout.pixel {
        DdsPixel::Format(PixelFormat::Argb8) => decode_uncompressed(
            width,
            height,
            32,
            [0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000],
            row_pitch,
            data,
        ),
        DdsPixel::Format(PixelFormat::R8) => decode_uncompressed(
            width,
            height,
            8,
            // DirectXTex splats the single channel into G and B.
            [0xff, 0xff, 0xff, 0],
            row_pitch,
            data,
        ),
        DdsPixel::Uncompressed {
            bit_count,
            r_mask,
            g_mask,
            b_mask,
            a_mask,
        } => decode_uncompressed(
            width,
            height,
            bit_count,
            [r_mask, g_mask, b_mask, a_mask],
            row_pitch,
            data,
        ),
        DdsPixel::Format(_) => {
            let (_, variant) = codec.ok_or(ConvertError::Unsupported("pixel format"))?;
            decompress_mip(variant, width, height, data)
        }
    }
}

/// Decompresses one mip's blocks. The decoder writes whole 4x4 blocks, so a
/// mip smaller than 4 on a side is decoded at its block-rounded size and
/// cropped; rows beyond the mip's own pitch are dropped.
fn decompress_mip(
    variant: CompressionVariant,
    width: u32,
    height: u32,
    blocks: &[u8],
) -> Result<Vec<u8>, ConvertError> {
    // The decoder takes u32 dimensions; the block grid is computed in u64
    // so a dimension at the type's edge cannot wrap.
    let padded_width =
        u32::try_from(u64::from(width).div_ceil(4) * 4).map_err(|_| ConvertError::Truncated)?;
    let padded_height =
        u32::try_from(u64::from(height).div_ceil(4) * 4).map_err(|_| ConvertError::Truncated)?;
    let padded_len = u64::from(padded_width)
        .checked_mul(u64::from(padded_height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ConvertError::Truncated)
        .and_then(alloc_len)?;
    let mut padded = vec![0u8; padded_len];
    decompress_blocks_as_rgba8(variant, padded_width, padded_height, blocks, &mut padded);

    // The decoder truncates the interpolated index values; DirectXTex's byte
    // store (2024.1.1.1, the fixtures' build) rounds them to nearest for BC3's
    // RGBA and BC5's R8G8 output but truncates for
    // BC4's R8, which matches what the decoder produced, so only BC3's alpha
    // and BC5's data channels are recomputed from the endpoints. DirectXTex
    // added the rounding bias to R8 in 2026 (#671); fixtures from a later
    // build would move BC4 into the recomputed set.
    fix_interpolated_channels(variant, padded_width, padded_height, blocks, &mut padded);

    let rgba_len = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ConvertError::Truncated)
        .and_then(alloc_len)?;
    let mut rgba = vec![0u8; rgba_len];
    for y in 0..height as usize {
        let from = y * padded_width as usize * 4;
        let to = y * width as usize * 4;
        rgba[to..to + width as usize * 4].copy_from_slice(&padded[from..from + width as usize * 4]);
    }
    // The block decoder leaves missing channels zero; DirectXTex reports the
    // one-channel format with R splatted into G and B, and both formats'
    // alpha as opaque (BC5 keeps B = 0).
    if let CompressionVariant::BC4 = variant {
        for pixel in rgba.as_chunks_mut::<4>().0.iter_mut() {
            pixel[1] = pixel[0];
            pixel[2] = pixel[0];
        }
    }
    if matches!(variant, CompressionVariant::BC4 | CompressionVariant::BC5) {
        for pixel in rgba.as_chunks_mut::<4>().0.iter_mut() {
            pixel[3] = 255;
        }
    }
    Ok(rgba)
}

/// Rewrites the alpha-indexed channels of `padded` with rounded
/// interpolation: BC3's alpha and BC5's data channels use the 8-byte
/// endpoint+3-bit-index format, whose interpolated entries are
/// `(e0*(n-i) + e1*i + n/2) / n`. BC4's store truncates, which the block
/// decoder already does, so it is not recomputed.
fn fix_interpolated_channels(
    variant: CompressionVariant,
    padded_width: u32,
    padded_height: u32,
    blocks: &[u8],
    padded: &mut [u8],
) {
    // (block byte size, alpha-block offsets within the block, output channel
    // per alpha block)
    let (block_bytes, alpha_blocks): (usize, &[(usize, usize)]) = match variant {
        CompressionVariant::BC3 => (16, &[(0, 3)]),
        CompressionVariant::BC5 => (16, &[(0, 0), (8, 1)]),
        _ => return,
    };
    let blocks_x = padded_width / 4;
    let blocks_y = padded_height / 4;
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block = &blocks[((by * blocks_x + bx) as usize * block_bytes)..];
            for &(alpha_offset, channel) in alpha_blocks {
                let alpha_block = &block[alpha_offset..alpha_offset + 8];
                let ramp = alpha_ramp(alpha_block[0], alpha_block[1]);
                let mut packed = [0u8; 8];
                packed[..6].copy_from_slice(&alpha_block[2..8]);
                let indices = u64::from_le_bytes(packed);
                for index in 0..16u32 {
                    let x = bx * 4 + index % 4;
                    let y = by * 4 + index / 4;
                    let at = ((y * padded_width + x) * 4) as usize + channel;
                    padded[at] = ramp[((indices >> (3 * index)) & 7) as usize];
                }
            }
        }
    }
}

/// The 8-entry value ramp of an alpha-indexed block: endpoints then the
/// interpolated entries, rounded to nearest as DirectXTex's R8G8 and RGBA
/// byte stores do.
fn alpha_ramp(a0: u8, a1: u8) -> [u8; 8] {
    let mut ramp = [a0, a1, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for i in 1..7u32 {
            ramp[1 + i as usize] = ((u32::from(a0) * (7 - i) + u32::from(a1) * i + 3) / 7) as u8;
        }
    } else {
        for i in 1..5u32 {
            ramp[1 + i as usize] = ((u32::from(a0) * (5 - i) + u32::from(a1) * i + 2) / 5) as u8;
        }
        ramp[6] = 0;
        ramp[7] = 255;
    }
    ramp
}

/// Expands one uncompressed mip to RGBA8 through its channel masks. Rows
/// are `row_pitch` bytes apart in the data. Every mask must be a
/// byte-aligned 8-bit field; anything else is rejected. A channel with no
/// mask reads 0; a missing alpha mask yields 255.
fn decode_uncompressed(
    width: u32,
    height: u32,
    bit_count: u32,
    masks: [u32; 4],
    row_pitch: usize,
    data: &[u8],
) -> Result<Vec<u8>, ConvertError> {
    let bytes_per_pixel = match bit_count {
        8 | 16 | 24 | 32 => (bit_count / 8) as usize,
        _ => return Err(ConvertError::Unsupported("pixel bit count")),
    };
    // Checked so a dimension product cannot wrap on a 32-bit target.
    let row_bytes = usize::try_from(u64::from(width) * bytes_per_pixel as u64)
        .map_err(|_| ConvertError::Truncated)?;
    // No channel masks at all leaves nothing to decode.
    if masks == [0; 4] {
        return Err(ConvertError::Unsupported("pixel masks"));
    }
    let mut channels = [None; 4];
    for (channel, mask) in masks.iter().enumerate() {
        if *mask == 0 {
            continue;
        }
        let shift = mask.trailing_zeros();
        if *mask != 0xff << shift || shift % 8 != 0 || shift >= bit_count {
            return Err(ConvertError::Unsupported("pixel masks"));
        }
        // `shift < bit_count` and a multiple of 8, so the byte index is
        // always inside the pixel.
        channels[channel] = Some(shift / 8);
    }

    let mut rgba = Vec::with_capacity(alloc_len(
        u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(4),
    )?);
    for y in 0..height as usize {
        let start = y.checked_mul(row_pitch).ok_or(ConvertError::Truncated)?;
        let end = start
            .checked_add(row_bytes)
            .ok_or(ConvertError::Truncated)?;
        let row = data.get(start..end).ok_or(ConvertError::Truncated)?;
        for pixel in row.chunks_exact(bytes_per_pixel) {
            for (channel, rgba_channel) in channels.iter().enumerate() {
                let value = match rgba_channel {
                    Some(byte) => pixel[*byte as usize],
                    _ if channel == 3 => 255,
                    _ => 0,
                };
                rgba.push(value);
            }
        }
    }
    Ok(rgba)
}
