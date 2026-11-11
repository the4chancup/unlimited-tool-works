//! In-process texture conversion: decode DDS/FTEX/raster sources, generate
//! mips, encode BC1/BC3/BC7, cache results.
//!
//! Three pure steps and one stateful wrapper: [`decode`] turns any accepted
//! source into straight-alpha RGBA8 mips plus, for DDS/FTEX, the compressed
//! blocks it carried and a flag saying the mip chain is the source's own;
//! [`convert`] applies the codec rules to a decoded texture and returns the
//! finished container bytes (a DDS for PES 15-17, an FTEX for PES 18-21);
//! [`Converter`] is the session cache in front of both. [`encode_dds`] is
//! the one exception to choosing by version: a file every engine reads as a
//! DDS in one fixed codec (a player portrait, BC3) names its codec.
//! [`probe`] reads a source's size from its header alone, for checks that
//! must not pay a decode per texture. [`resize`] is the suite's one
//! resampler (Lanczos3). [`encode_png`] is the one raster the crate writes:
//! the game reads a team's logo as PNGs. On native targets [`convert_with_gpu`]
//! runs the same conversion with its BC7 encodes on a [`gpu::GpuEncoder`],
//! falling back to the CPU per texture.

mod cache;
mod dds;
mod encode;
#[cfg(not(target_arch = "wasm32"))]
pub mod gpu;
/// The wasm build has no GPU encoder: an uninhabited stand-in lets the conversion
/// code take an `Option<&mut GpuEncoder>` on every target, always `None` there.
#[cfg(target_arch = "wasm32")]
mod gpu {
    use block_compression::CompressionVariant;

    use crate::ConvertError;

    /// No GPU encoder exists on wasm; `encode` never sees one.
    pub(crate) enum GpuEncoder {}

    impl GpuEncoder {
        pub(crate) fn accepts(
            &self,
            _variant: CompressionVariant,
            _width: u32,
            _height: u32,
        ) -> bool {
            match *self {}
        }
        pub(crate) fn encode(
            &mut self,
            _variant: CompressionVariant,
            _width: u32,
            _height: u32,
            _rgba: &[u8],
        ) -> Result<Vec<u8>, ConvertError> {
            match *self {}
        }
    }
}
mod jpeg_xl;
mod mips;
mod raster;
mod resample;

use pes_version::PesVersion;
use sha2::{Digest, Sha256};

pub use cache::Converter;
pub use raster::encode_png;
pub use resample::{resize, resize_straight_alpha};

/// The accepted source formats, named by the file extension the compiler
/// resolved the texture stem to. TGA has no magic, so the format is never
/// sniffed from bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFormat {
    /// A DDS file (optionally WESYS-wrapped for PES 15-17 sources).
    Dds,
    /// A Fox Engine FTEX file.
    Ftex,
    /// `.png`
    Png,
    /// `.jpg` / `.jpeg`
    Jpeg,
    /// `.bmp`
    Bmp,
    /// `.webp`
    WebP,
    /// `.tga`
    Tga,
    /// `.tif` / `.tiff`
    Tiff,
    /// `.jxl` (JPEG XL)
    Jxl,
}

impl SourceFormat {
    /// Case-insensitive, without the dot; `jpg`/`jpeg` and `tif`/`tiff`
    /// both accepted.
    pub fn from_extension(extension: &str) -> Option<Self> {
        Some(match extension.to_ascii_lowercase().as_str() {
            "dds" => SourceFormat::Dds,
            "ftex" => SourceFormat::Ftex,
            "png" => SourceFormat::Png,
            "jpg" | "jpeg" => SourceFormat::Jpeg,
            "bmp" => SourceFormat::Bmp,
            "webp" => SourceFormat::WebP,
            "tga" => SourceFormat::Tga,
            "tif" | "tiff" => SourceFormat::Tiff,
            "jxl" => SourceFormat::Jxl,
            _ => return None,
        })
    }
}

/// What the texture is for; with the PES version it selects the codec and
/// the channel layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureRole {
    /// An ordinary color texture.
    Color,
    /// A normal map (encoded in the DXT5nm channel layout).
    Normal,
}

/// Block codecs this crate keeps or emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockCodec {
    /// BC1 / DXT1.
    Bc1,
    /// BC2 / DXT3.
    Bc2,
    /// BC3 / DXT5.
    Bc3,
    /// BC4 / ATI1 (single channel).
    Bc4,
    /// BC5 / ATI2 (two channels; normal-map sources).
    Bc5,
    /// BC7 (PES 19-21 only).
    Bc7,
}

/// Compressed blocks a DDS/FTEX source carried, one buffer per mip, kept so
/// a target that reads the codec gets them unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocks {
    /// The codec the blocks are in.
    pub codec: BlockCodec,
    /// One buffer per mip, top level first.
    pub mips: Vec<Vec<u8>>,
}

/// A source decoded to straight-alpha RGBA8, top mip first, every mip the
/// source carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// Level-0 width in pixels.
    pub width: u32,
    /// Level-0 height in pixels.
    pub height: u32,
    /// RGBA8 pixels, one buffer per mip, tightly packed row-major.
    pub mips: Vec<Vec<u8>>,
    /// The compressed blocks the source carried, when it was a DDS/FTEX in
    /// a block codec this crate knows.
    pub blocks: Option<Blocks>,
    /// True when the source format carries a mip chain (DDS/FTEX), so its
    /// level count is kept on encode; false for raster sources, which get a
    /// generated chain down to 1x1.
    pub authored_mips: bool,
    /// True when the source was a DDS or FTEX stored uncompressed (any
    /// layout the crate reads, R8/L8 included); false for block-compressed
    /// and raster sources.
    pub uncompressed: bool,
}

/// The version and role a conversion targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target {
    /// The PES version the texture is compiled for.
    pub version: PesVersion,
    /// Whether the texture is color data or a normal map.
    pub role: TextureRole,
    /// Whether an uncompressed source is written uncompressed (in the
    /// A8R8G8B8 layout); `false` asks for an encode.
    pub keep_uncompressed: bool,
}

/// Decodes an accepted source to RGBA8 mips plus any compressed blocks it
/// carried. Cube maps and volume textures are rejected
/// (`ConvertError::Ftex` wrapping `ftex`'s `UnsupportedDds`); a
/// WESYS-wrapped DDS is unwrapped first.
pub fn decode(bytes: &[u8], format: SourceFormat) -> Result<Decoded, ConvertError> {
    match format {
        SourceFormat::Dds => {
            let unwrapped = wezlib::decompress_if_wrapped(bytes)?;
            dds::decode_dds(&unwrapped)
        }
        SourceFormat::Ftex => dds::decode_ftex(bytes),
        SourceFormat::Png
        | SourceFormat::Jpeg
        | SourceFormat::Bmp
        | SourceFormat::WebP
        | SourceFormat::Tga
        | SourceFormat::Tiff => raster::decode_raster(bytes, format),
        SourceFormat::Jxl => jpeg_xl::decode(bytes),
    }
}

/// Applies the codec rules to a decoded texture: compatible compressed
/// blocks pass through with only a container rebuild, an uncompressed
/// source whose target keeps it is written uncompressed, everything else
/// is re-encoded per target and role. Returns a DDS for PES 15-17, an FTEX
/// for PES 18-21.
///
/// # Errors
///
/// `ConvertError::InvalidDecoded` when a caller-built `Decoded` breaks the
/// rules a `decode` product follows (zero dimensions, missing or
/// undersized mips, block buffers of the wrong size).
pub fn convert(decoded: &Decoded, target: Target) -> Result<Vec<u8>, ConvertError> {
    validate(decoded)?;
    encode::convert(decoded, target, None)
}

/// `convert`, with the BC7 encodes on `gpu` while it is usable. A texture the
/// GPU cannot take (larger than the device's texture limit) is encoded on the
/// CPU and the encoder stays usable; a failure on the device is logged, makes
/// `gpu.unavailable()` return its reason, and the mip it happened on and
/// everything after it (in that texture and in later ones) is encoded on the
/// CPU. Output bytes may differ from `convert`'s, and across GPUs and drivers:
/// only the CPU encode is byte-reproducible. The container layout, dimensions,
/// mip count and mips' uncompressed sizes never differ; an FTEX's compressed
/// sizes and total length may.
///
/// # Errors
///
/// As [`convert`].
#[cfg(not(target_arch = "wasm32"))]
pub fn convert_with_gpu(
    decoded: &Decoded,
    target: Target,
    gpu: &mut gpu::GpuEncoder,
) -> Result<Vec<u8>, ConvertError> {
    validate(decoded)?;
    encode::convert(decoded, target, Some(gpu))
}

/// A DDS in `codec` (`Bc1`, `Bc3` or `Bc7`; any other is `Unsupported`) at the source's own
/// size, whatever the target engine, color layout: the source's mip count kept, a raster
/// source's chain generated, a block source already in `codec` keeping its blocks.
///
/// # Errors
///
/// `ConvertError::InvalidDecoded` as [`convert`]; `ConvertError::Unsupported` for a codec
/// this crate does not encode.
pub fn encode_dds(decoded: &Decoded, codec: BlockCodec) -> Result<Vec<u8>, ConvertError> {
    validate(decoded)?;
    encode::encode_dds(decoded, codec)
}

/// What a source's header says of its texture, read without decoding a pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// Level-0 width in pixels.
    pub width: u32,
    /// Level-0 height in pixels.
    pub height: u32,
    /// The level count the converted texture carries: a DDS or FTEX
    /// source's own, a raster source's full chain down to 1x1, which
    /// conversion generates.
    pub mipmaps: u32,
}

/// Reads a source's size and level count from its header alone, for checks
/// that must not pay a decode per texture: a DDS's (WESYS-wrapped or not)
/// through its DDS header, an FTEX's through its FTEX header, a raster
/// source's through its decoder's header read.
///
/// # Errors
///
/// What [`decode`] returns for a header it cannot read. A texture that
/// probes can still fail to decode: the pixel data is not read.
pub fn probe(bytes: &[u8], format: SourceFormat) -> Result<Probe, ConvertError> {
    match format {
        SourceFormat::Dds => {
            let unwrapped = wezlib::decompress_if_wrapped(bytes)?;
            let layout = ftex::dds::read_layout(&unwrapped)?;
            Ok(Probe {
                width: layout.width,
                height: layout.height,
                mipmaps: layout.mipmaps,
            })
        }
        SourceFormat::Ftex => {
            let info = ftex::info(bytes)?;
            Ok(Probe {
                width: u32::from(info.width),
                height: u32::from(info.height),
                mipmaps: u32::from(info.mipmaps),
            })
        }
        SourceFormat::Png
        | SourceFormat::Jpeg
        | SourceFormat::Bmp
        | SourceFormat::WebP
        | SourceFormat::Tga
        | SourceFormat::Tiff => Ok(raster_probe(raster::dimensions(bytes, format)?)),
        SourceFormat::Jxl => Ok(raster_probe(jpeg_xl::dimensions(bytes)?)),
    }
}

/// The probe of a source that carries one level, `(width, height)`: the
/// chain conversion generates for it is the full one.
fn raster_probe((width, height): (u32, u32)) -> Probe {
    Probe {
        width,
        height,
        mipmaps: mips::chain_len(width, height),
    }
}

/// The `usize` length of a buffer a texture dimension declares. On a
/// 32-bit target a length past `isize::MAX` would panic with a capacity
/// overflow where it should be a refusal.
pub(crate) fn alloc_len(len: u64) -> Result<usize, ConvertError> {
    if len > isize::MAX as u64 {
        return Err(ConvertError::Unsupported(
            "texture larger than this target can hold",
        ));
    }
    usize::try_from(len)
        .map_err(|_| ConvertError::Unsupported("texture larger than this target can hold"))
}

/// Refuses a caller-built `Decoded` that `decode` could not produce.
fn validate(decoded: &Decoded) -> Result<(), ConvertError> {
    if decoded.width == 0 || decoded.height == 0 {
        return Err(ConvertError::InvalidDecoded("zero dimension"));
    }
    // mips::downsample and the block codecs index pixels in u32: a top
    // level past 4 GiB of RGBA has no valid downstream representation.
    let top_level_rgba = u64::from(decoded.width)
        .saturating_mul(u64::from(decoded.height))
        .saturating_mul(4);
    if top_level_rgba > u64::from(u32::MAX) {
        return Err(ConvertError::InvalidDecoded("past 4 GiB"));
    }
    if decoded.mips.is_empty() {
        return Err(ConvertError::InvalidDecoded("no mips"));
    }
    if !decoded.authored_mips && decoded.mips.len() > 1 {
        return Err(ConvertError::InvalidDecoded("unauthored mip chain"));
    }
    // A mip chain bottoms out at 1x1: log2(max side) + 1 levels.
    if decoded.mips.len() as u32 > 32 - decoded.width.max(decoded.height).leading_zeros() {
        return Err(ConvertError::InvalidDecoded(
            "mip count past the dimensions",
        ));
    }
    for (level, mip) in decoded.mips.iter().enumerate() {
        let w = decoded.width.checked_shr(level as u32).unwrap_or(0).max(1);
        let h = decoded.height.checked_shr(level as u32).unwrap_or(0).max(1);
        let expected = u64::from(w)
            .checked_mul(u64::from(h))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(ConvertError::InvalidDecoded("mip size"))?;
        if mip.len() as u64 != expected {
            return Err(ConvertError::InvalidDecoded("mip size"));
        }
    }
    if let Some(blocks) = &decoded.blocks {
        if blocks.mips.len() != decoded.mips.len() {
            return Err(ConvertError::InvalidDecoded("block mip count"));
        }
        let format = encode::pixel_format(blocks.codec);
        for (level, mip) in blocks.mips.iter().enumerate() {
            if mip.len() != ftex::mip_size(format, decoded.width, decoded.height, 1, level as u32) {
                return Err(ConvertError::InvalidDecoded("block mip size"));
            }
        }
    }
    Ok(())
}

/// SHA-256 of the source bytes, computed once when the file is
/// materialized and reused as the conversion cache's key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceHash(pub [u8; 32]);

/// Hashes `bytes` for the conversion cache.
pub fn source_hash(bytes: &[u8]) -> SourceHash {
    SourceHash(Sha256::digest(bytes).into())
}

/// Whether [`Converter::convert`] reads and writes the session cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CachePolicy {
    /// Cache hit returns the retained bytes; a miss inserts.
    Use,
    /// No read, no insert (large compiles bypass the cache entirely).
    Bypass,
}

/// Why a source could not be decoded or converted.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// The DDS/FTEX container could not be read or written.
    #[error("container conversion failed: {0}")]
    Ftex(#[from] ftex::FtexError),
    /// A WESYS-wrapped DDS could not be unwrapped.
    #[error("wesys unwrap failed: {0}")]
    Wesys(#[from] wezlib::Error),
    /// A raster source could not be decoded, or [`encode_png`]'s encoder
    /// failed.
    #[error("image decode failed: {0}")]
    Image(#[from] image::ImageError),
    /// A feature of the source or request this crate does not handle.
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    /// A caller-built `Decoded` breaks the rules a decoded one follows, or
    /// the pixels handed to [`resize`] or [`encode_png`] do not match their
    /// size.
    #[error("inconsistent decoded texture: {0}")]
    InvalidDecoded(&'static str),
    /// The buffer ends before a structure that extends past it.
    #[error("buffer is truncated")]
    Truncated,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    const BC1: &[u8] = include_bytes!("../tests/fixtures/bc1_opaque.dds");
    const BC1_D: &[u8] = include_bytes!("../tests/fixtures/bc1_opaque.decoded.dds");
    const BC3: &[u8] = include_bytes!("../tests/fixtures/bc3.dds");
    const BC3_D: &[u8] = include_bytes!("../tests/fixtures/bc3.decoded.dds");
    const BC5: &[u8] = include_bytes!("../tests/fixtures/bc5.dds");
    const BC5_D: &[u8] = include_bytes!("../tests/fixtures/bc5.decoded.dds");
    const ATI2: &[u8] = include_bytes!("../tests/fixtures/ati2.dds");
    const ATI2_D: &[u8] = include_bytes!("../tests/fixtures/ati2.decoded.dds");
    const BC7: &[u8] = include_bytes!("../tests/fixtures/bc7.dds");
    const BC7_D: &[u8] = include_bytes!("../tests/fixtures/bc7.decoded.dds");
    const RGBA8: &[u8] = include_bytes!("../tests/fixtures/rgba8.dds");
    const RGBA8_D: &[u8] = include_bytes!("../tests/fixtures/rgba8.decoded.dds");
    const BGRA8: &[u8] = include_bytes!("../tests/fixtures/bgra8_dx9.dds");
    const BGRA8_D: &[u8] = include_bytes!("../tests/fixtures/bgra8_dx9.decoded.dds");
    const BC2: &[u8] = include_bytes!("../tests/fixtures/bc2.dds");
    const BC2_D: &[u8] = include_bytes!("../tests/fixtures/bc2.decoded.dds");
    const BC4: &[u8] = include_bytes!("../tests/fixtures/bc4.dds");
    const BC4_D: &[u8] = include_bytes!("../tests/fixtures/bc4.decoded.dds");
    const ATI1: &[u8] = include_bytes!("../tests/fixtures/ati1.dds");
    const ATI1_D: &[u8] = include_bytes!("../tests/fixtures/ati1.decoded.dds");
    const R8_DDS: &[u8] = include_bytes!("../tests/fixtures/r8.dds");
    const R8_D: &[u8] = include_bytes!("../tests/fixtures/r8.decoded.dds");
    const L8: &[u8] = include_bytes!("../tests/fixtures/l8_dx9.dds");
    const L8_D: &[u8] = include_bytes!("../tests/fixtures/l8_dx9.decoded.dds");
    const BC4_EQ: &[u8] = include_bytes!("../tests/fixtures/bc4_equal_endpoints.dds");
    const BC4_EQ_D: &[u8] = include_bytes!("../tests/fixtures/bc4_equal_endpoints.decoded.dds");
    const BGR24: &[u8] = include_bytes!("../tests/fixtures/bgr24_dword_rows.dds");
    const BGR24_D: &[u8] = include_bytes!("../tests/fixtures/bgr24_dword_rows.decoded.dds");
    const L8_NVTT: &[u8] = include_bytes!("../tests/fixtures/l8_nvtt1.dds");
    const L8_NVTT_D: &[u8] = include_bytes!("../tests/fixtures/l8_nvtt1.decoded.dds");
    const TIFF_ASSOCIATED: &[u8] = include_bytes!("../tests/fixtures/rgba_associated.tiff");
    const TIFF_UNASSOCIATED: &[u8] = include_bytes!("../tests/fixtures/rgba_unassociated.tiff");
    const NM_PREFOX_D: &[u8] = include_bytes!("../tests/fixtures/bc3_nm_prefox.decoded.dds");
    const PNG: &[u8] = include_bytes!("../tests/fixtures/source.png");
    const PNG_OPAQUE: &[u8] = include_bytes!("../tests/fixtures/source_opaque.png");
    const JXL_LOSSLESS: &[u8] = include_bytes!("../tests/fixtures/source.lossless.jxl");
    const JXL_CONTAINER: &[u8] = include_bytes!("../tests/fixtures/source.lossless.container.jxl");
    const JXL_LOSSY: &[u8] = include_bytes!("../tests/fixtures/source.lossy.jxl");
    const FTEX_BC1: &[u8] = include_bytes!("../tests/fixtures/konami_bc1_bibs_metalness.ftex");
    const DDS_BC1: &[u8] = include_bytes!("../tests/fixtures/konami_bc1_bibs_metalness.dds");
    const CUBE: &[u8] =
        include_bytes!("../tests/fixtures/konami_bc1_cubemap_default_reflection.dds");
    const BC7_PES21: &[u8] = include_bytes!("../tests/fixtures/bc7.pes21.ftex");
    const BC3_WESYS_PES17: &[u8] = include_bytes!("../tests/fixtures/bc3_wesys.pes17.dds");

    fn mips_of(dds: &[u8]) -> Vec<Vec<u8>> {
        decode(dds, SourceFormat::Dds).unwrap().mips
    }

    /// Per mip, the largest absolute difference per channel.
    fn max_per_channel(expected: &[Vec<u8>], actual: &[Vec<u8>]) -> Vec<[u8; 4]> {
        assert_eq!(expected.len(), actual.len(), "mip count");
        expected
            .iter()
            .zip(actual)
            .map(|(expected_mip, actual_mip)| {
                assert_eq!(expected_mip.len(), actual_mip.len(), "mip size");
                let mut max = [0u8; 4];
                for (expected_px, actual_px) in expected_mip
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(actual_mip.as_chunks::<4>().0.iter())
                {
                    for c in 0..4 {
                        max[c] = max[c].max(expected_px[c].abs_diff(actual_px[c]));
                    }
                }
                max
            })
            .collect()
    }

    /// Mean absolute difference over every byte of every mip.
    fn mean_error(expected: &[Vec<u8>], actual: &[Vec<u8>]) -> f64 {
        let (sum, count) = expected.iter().zip(actual).fold(
            (0u64, 0u64),
            |(sum, count), (expected_mip, actual_mip)| {
                let mip_sum: u64 = expected_mip
                    .iter()
                    .zip(actual_mip)
                    .map(|(x, y)| u64::from(x.abs_diff(*y)))
                    .sum();
                (sum + mip_sum, count + expected_mip.len() as u64)
            },
        );
        sum as f64 / count as f64
    }

    /// The encode-quality standard: our encoder, measured against what its
    /// input decodes to, is no worse than the reference encoder measured the
    /// same way on the same image: per mip, the worst channel within one
    /// 5-bit quantization step (8) of the reference's worst channel; over the
    /// chain, the pixel-weighted mean within 1.0 (the top mip dominates it,
    /// as it does the screen). A fixed tolerance would be meaningless here:
    /// the test image packs a 2D gradient and a checker into single blocks
    /// at the small mips, where a BC1 colour line loses over 100 in some
    /// channel for any encoder, the reference included; which channel is
    /// sacrificed is the encoder's choice, hence worst channel rather than
    /// channel by channel. Per-mip means are not compared: the CPU BC1 core
    /// (PCA plus one refinement pass) trails the reference by about 2.5 on
    /// the 8x4 mip of a grey ramp, a known gap.
    fn assert_not_worse_than_reference(
        label: &str,
        truth: &[Vec<u8>],
        ours: &[Vec<u8>],
        reference_truth: &[Vec<u8>],
        reference: &[Vec<u8>],
    ) {
        let ours_max = max_per_channel(truth, ours);
        let reference_max = max_per_channel(reference_truth, reference);
        for (level, (o, r)) in ours_max.iter().zip(&reference_max).enumerate() {
            let (ours_worst, reference_worst) = (o.iter().max(), r.iter().max());
            assert!(
                ours_worst
                    <= reference_worst
                        .map(|worst| worst.saturating_add(8))
                        .as_ref(),
                "{label} mip {level}: worst channel ours {ours_worst:?} vs reference {reference_worst:?}"
            );
        }
        let ours_mean = mean_error(truth, ours);
        let reference_mean = mean_error(reference_truth, reference);
        assert!(
            ours_mean <= reference_mean + 1.0,
            "{label} mean: ours {ours_mean:.3} vs reference {reference_mean:.3}"
        );
    }

    /// The DXT5nm layouts applied to plain RGBA mips (X = R, Y = G).
    fn dxt5nm(mips: &[Vec<u8>], fox: bool) -> Vec<Vec<u8>> {
        mips.iter()
            .map(|mip| {
                mip.as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|px| {
                        let (x, y) = (px[0], px[1]);
                        if fox { [255, y, 0, x] } else { [y, y, y, x] }
                    })
                    .collect()
            })
            .collect()
    }

    /// Reads a pre-Fox DXT5nm decode back as plain (X = R, Y = G) pixels.
    fn plain_from_prefox(prefox: &[Vec<u8>]) -> Vec<Vec<u8>> {
        prefox
            .iter()
            .map(|mip| {
                mip.as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|px| [px[3], px[1], 0, 255])
                    .collect()
            })
            .collect()
    }

    #[test]
    fn decode_matches_the_reference_decode_exactly() {
        for (stem, encoded, decoded) in [
            ("bc1_opaque", BC1, BC1_D),
            ("bc2", BC2, BC2_D),
            ("bc3", BC3, BC3_D),
            ("bc4", BC4, BC4_D),
            ("ati1", ATI1, ATI1_D),
            ("bc5", BC5, BC5_D),
            ("ati2", ATI2, ATI2_D),
            ("bc7", BC7, BC7_D),
            ("r8", R8_DDS, R8_D),
            ("l8_dx9", L8, L8_D),
            ("l8_nvtt1", L8_NVTT, L8_NVTT_D),
            ("rgba8", RGBA8, RGBA8_D),
            ("bgra8_dx9", BGRA8, BGRA8_D),
        ] {
            let ours = decode(encoded, SourceFormat::Dds).unwrap();
            let expected = decode(decoded, SourceFormat::Dds).unwrap();
            assert_eq!(
                (ours.width, ours.height, ours.mips.len()),
                (expected.width, expected.height, expected.mips.len()),
                "{stem}"
            );
            for (level, (ours_mip, expected_mip)) in
                ours.mips.iter().zip(&expected.mips).enumerate()
            {
                assert_eq!(
                    ours_mip.len(),
                    expected_mip.len(),
                    "{stem} mip {level} length"
                );
                let diffs: Vec<(usize, u8, u8)> = ours_mip
                    .iter()
                    .zip(expected_mip)
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, (a, b))| (i, *a, *b))
                    .collect();
                assert!(
                    diffs.is_empty(),
                    "{stem} mip {level}: {} diffs, first {diffs:?}",
                    diffs.len()
                );
            }
        }
    }

    #[test]
    fn an_nvtt1_l8_header_reads_as_r8() {
        assert_eq!(
            ftex::dds::read_layout(L8_NVTT).unwrap().pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::R8)
        );
    }

    #[test]
    fn tiff_associated_alpha_is_un_premultiplied() {
        // ExtraSamples = 1 marks the stored RGBA premultiplied; the decode
        // restores straight alpha. ExtraSamples = 2 (or none) is copied
        // through unchanged.
        let associated = decode(TIFF_ASSOCIATED, SourceFormat::Tiff).unwrap();
        assert_eq!(associated.mips[0], [255, 0, 0, 128, 0, 0, 0, 0]);
        let unassociated = decode(TIFF_UNASSOCIATED, SourceFormat::Tiff).unwrap();
        assert_eq!(unassociated.mips[0], [128, 0, 0, 128, 0, 0, 0, 0]);
    }

    #[test]
    fn tiff_un_premultiply_rounds_to_nearest() {
        use std::io::Cursor;
        use tiff::encoder::{TiffEncoder, colortype};
        use tiff::tags::Tag;
        // A stored premultiplied channel of 1 under alpha 2 un-multiplies
        // to 128 (255/2 rounding up), not 127: the half-alpha bias in the
        // divide is what makes the difference.
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut tiff = TiffEncoder::new(&mut bytes).unwrap();
            let mut image = tiff.new_image::<colortype::RGBA8>(1, 1).unwrap();
            image.encoder().write_tag(Tag::ExtraSamples, 1u16).unwrap();
            image.write_data(&[1, 0, 0, 2]).unwrap();
        }
        let decoded = decode(&bytes.into_inner(), SourceFormat::Tiff).unwrap();
        assert_eq!(decoded.mips[0], [128, 0, 0, 2]);
    }

    #[test]
    fn tiff_associated_alpha_un_premultiplies_at_source_precision() {
        use std::io::Cursor;
        use tiff::encoder::{TiffEncoder, colortype};
        use tiff::tags::Tag;
        // 16-bit premultiplied: the channel 128 under alpha 256 is 0x8000
        // straight at 16 bits; reducing to 8 bits first would have zeroed
        // the channel before the divide. The second pixel's half-alpha
        // bias changes the rounded result by a step.
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut tiff = TiffEncoder::new(&mut bytes).unwrap();
            let mut image = tiff.new_image::<colortype::RGBA16>(2, 1).unwrap();
            image.encoder().write_tag(Tag::ExtraSamples, 1u16).unwrap();
            image
                .write_data(&[128u16, 0, 0, 256, 4, 0, 0, 157])
                .unwrap();
        }
        let decoded = decode(&bytes.into_inner(), SourceFormat::Tiff).unwrap();
        assert_eq!(decoded.mips[0], [128, 0, 0, 1, 6, 0, 0, 1]);
    }

    #[test]
    fn tiff_un_premultiplies_8_bit_at_8_bit_precision() {
        use std::io::Cursor;
        use tiff::encoder::{TiffEncoder, colortype};
        use tiff::tags::Tag;
        // (1, 6) un-multiplies to 43 in the u8 formula.
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut tiff = TiffEncoder::new(&mut bytes).unwrap();
            let mut image = tiff.new_image::<colortype::RGBA8>(1, 1).unwrap();
            image.encoder().write_tag(Tag::ExtraSamples, 1u16).unwrap();
            image.write_data(&[1, 0, 0, 6]).unwrap();
        }
        let decoded = decode(&bytes.into_inner(), SourceFormat::Tiff).unwrap();
        assert_eq!(decoded.mips[0], [43, 0, 0, 6]);
    }

    #[test]
    fn bc4u_fourcc_decodes_as_bc4() {
        // DirectXTex's DDSPF_BC4_UNORM spells the FourCC BC4U where the
        // fixture says ATI1; the blocks are the same.
        let mut dds = ATI1.to_vec();
        dds[84..88].copy_from_slice(b"BC4U");
        assert_eq!(
            decode(&dds, SourceFormat::Dds).unwrap().mips,
            decode(ATI1_D, SourceFormat::Dds).unwrap().mips
        );
    }

    #[test]
    fn tiff_float_associated_alpha_un_premultiplies_in_f32() {
        use std::io::Cursor;
        use tiff::encoder::{TiffEncoder, colortype};
        use tiff::tags::Tag;
        // The channel 5e-6 under alpha 1e-5 is 0.5 straight; quantizing to
        // u16 or u8 first would have zeroed it before the divide. A zero
        // alpha leaves the stored channel as decoded.
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut tiff = TiffEncoder::new(&mut bytes).unwrap();
            let mut image = tiff.new_image::<colortype::RGBA32Float>(2, 1).unwrap();
            image.encoder().write_tag(Tag::ExtraSamples, 1u16).unwrap();
            image
                .write_data(&[5e-6f32, 0.0, 0.0, 1e-5, 0.5, 0.0, 0.0, 0.0])
                .unwrap();
        }
        let decoded = decode(&bytes.into_inner(), SourceFormat::Tiff).unwrap();
        assert_eq!(decoded.mips[0], [128, 0, 0, 0, 128, 0, 0, 0]);
    }

    #[test]
    fn a_16_bit_tga_is_refused() {
        // A 2x1 truecolor TGA at 16 bits holds 5-bit primaries plus one
        // alpha bit the `image` decoder drops.
        let mut tga = vec![
            0, 0, 2, // no id, no colormap, truecolor
            0, 0, 0, 0, 0, // colormap spec
            0, 0, 0, 0, // origin
            2, 0, 1, 0, // 2x1
            16, 0x21, // 16 bits, top-left, one attribute bit
        ];
        tga.extend_from_slice(&[0x00, 0x7c, 0x00, 0xfc]); // two red pixels
        assert!(matches!(
            decode(&tga, SourceFormat::Tga),
            Err(ConvertError::Unsupported("16-bit tga: resave as 32-bit"))
        ));
    }

    fn premultiplied_tga(attributes_type: u8, extension_offset: u32) -> Vec<u8> {
        // A 1x1 32-bit TGA 2.0 file: 18-byte header, one BGRA pixel, a
        // 495-byte extension area and the 26-byte footer.
        let mut tga = vec![
            0, 0, 2, // no id, no colormap, truecolor
            0, 0, 0, 0, 0, // colormap spec
            0, 0, 0, 0, // origin
            1, 0, 1, 0, // 1x1
            32, 0x28, // 32 bits, top-left, eight attribute bits
        ];
        tga.extend_from_slice(&[0, 0, 128, 128]); // BGRA, premultiplied red
        assert_eq!(tga.len(), 22);
        let mut extension = vec![0u8; 495];
        extension[0..2].copy_from_slice(&495u16.to_le_bytes()); // wSize
        extension[494] = attributes_type;
        tga.extend_from_slice(&extension);
        tga.extend_from_slice(&extension_offset.to_le_bytes());
        tga.extend_from_slice(&0u32.to_le_bytes()); // developer dir
        tga.extend_from_slice(b"TRUEVISION-XFILE.\0");
        tga
    }

    #[test]
    fn premultiplied_tga_is_un_premultiplied() {
        // Extension attributes type 4 declares premultiplied alpha: the
        // premultiplied red of 128 under alpha 128 straightens to 255.
        let tga = premultiplied_tga(4, 22);
        assert_eq!(
            decode(&tga, SourceFormat::Tga).unwrap().mips[0],
            [255, 0, 0, 128]
        );
        // Type 3 (unassociated) keeps the stored values.
        let unassociated = premultiplied_tga(3, 22);
        assert_eq!(
            decode(&unassociated, SourceFormat::Tga).unwrap().mips[0],
            [128, 0, 0, 128]
        );
        // An extension offset past the end of the file is not
        // premultiplied, and does not panic.
        let dangling = premultiplied_tga(4, 1 << 20);
        assert_eq!(
            decode(&dangling, SourceFormat::Tga).unwrap().mips[0],
            [128, 0, 0, 128]
        );
    }

    #[test]
    fn a_tga_declaration_past_the_allocation_limit_is_refused() {
        // A 16384x16384 header declares 1 GiB of RGBA: the allocation
        // limit image::ImageReader::decode applies refuses before
        // allocating, whatever pixel bytes follow.
        let tga = vec![
            0, 0, 2, // no id, no colormap, truecolor
            0, 0, 0, 0, 0, // colormap spec
            0, 0, 0, 0, // origin
            0, 64, 0, 64, // 16384x16384
            32, 0x28, // 32 bits, top-left, eight attribute bits
        ];
        assert!(matches!(
            decode(&tga, SourceFormat::Tga),
            Err(ConvertError::Image(image::ImageError::Limits(_)))
        ));
    }

    #[test]
    fn an_interleaved_tga_is_refused() {
        // Descriptor bits 6-7 declare interleaving DirectXTex refuses
        // (DirectXTexTGA.cpp:159-162); image decodes storage order.
        let mut tga = vec![
            0, 0, 2, // no id, no colormap, truecolor
            0, 0, 0, 0, 0, // colormap spec
            0, 0, 0, 0, // origin
            1, 0, 4, 0, // 1x4
            24, 0x60, // 24 bits, two-way interleaved
        ];
        tga.extend_from_slice(&[
            0, 0, 255, // red
            255, 0, 0, // blue
            0, 255, 0, // green
            255, 255, 255, // white
        ]);
        assert!(matches!(
            decode(&tga, SourceFormat::Tga),
            Err(ConvertError::Unsupported("interleaved tga"))
        ));
    }

    #[test]
    fn an_all_zero_alpha_tga_is_opaque() {
        // An all-zero alpha channel comes from a TGA writer that stored no alpha:
        // the decode reports opaque.
        let mut tga = vec![
            0, 0, 2, // no id, no colormap, truecolor
            0, 0, 0, 0, 0, // colormap spec
            0, 0, 0, 0, // origin
            2, 0, 1, 0, // 2x1
            32, 0x28, // 32 bits, top-left, eight attribute bits
        ];
        tga.extend_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0]); // BGRA
        assert_eq!(
            decode(&tga, SourceFormat::Tga).unwrap().mips[0],
            [255, 0, 0, 255, 0, 255, 0, 255]
        );
        // One nonzero alpha keeps the alphas as stored.
        tga.truncate(tga.len() - 4);
        tga.extend_from_slice(&[0, 255, 0, 1]);
        assert_eq!(
            decode(&tga, SourceFormat::Tga).unwrap().mips[0],
            [255, 0, 0, 0, 0, 255, 0, 1]
        );
    }

    #[test]
    fn an_l8_header_with_an_undeclared_alpha_mask_decodes_like_l8() {
        // DirectXTex's LUMINANCE match ignores a mask no flag declares
        // (DDS.cpp:274-279): a patched-in alpha mask changes nothing.
        let mut patched = L8_NVTT.to_vec();
        patched[104..108].copy_from_slice(&0xff000000u32.to_le_bytes());
        assert_eq!(
            decode(&patched, SourceFormat::Dds).unwrap().mips[0],
            decode(L8_NVTT, SourceFormat::Dds).unwrap().mips[0]
        );
    }

    #[test]
    fn a_top_level_past_the_codecs_4gib_indexing_is_refused() {
        // 32768x32770 needs more than 4 GiB of RGBA: mips::downsample and
        // the block codecs index in u32, so the declaration is invalid
        // outright rather than a size mismatch.
        let decoded = Decoded {
            width: 32768,
            height: 32770,
            mips: vec![vec![]],
            blocks: None,
            authored_mips: false,
            uncompressed: false,
        };
        assert!(matches!(
            convert(
                &decoded,
                Target {
                    version: PesVersion::Pes17,
                    role: TextureRole::Color,
                    keep_uncompressed: true,
                }
            ),
            Err(ConvertError::InvalidDecoded("past 4 GiB"))
        ));
    }

    #[test]
    fn alloc_len_stops_at_the_targets_index_limit() {
        // A buffer's length must fit isize::MAX on every target (wasm32's
        // usize is 4 GiB wide but its capacity limit is 2 GiB - 1): the
        // boundary itself is what the helper enforces.
        assert_eq!(alloc_len(isize::MAX as u64).unwrap(), isize::MAX as usize);
        assert!(matches!(
            alloc_len(isize::MAX as u64 + 1),
            Err(ConvertError::Unsupported(
                "texture larger than this target can hold"
            ))
        ));
    }

    #[test]
    fn a_24_bit_header_with_argb8_masks_is_not_argb8() {
        // DirectXTex matches bit counts before masks (DDS.cpp:222): the
        // canonical mask set on a 24-bit pixel is the generic layout,
        // whose fourth channel then sits past the pixel and is refused.
        let mut dds = uncompressed_dds(2, 1, 0, 24, [0xff0000, 0xff00, 0xff, 0xff000000], 1);
        dds[80..84].copy_from_slice(&0x41u32.to_le_bytes()); // RGB | ALPHAPIXELS
        dds.extend_from_slice(&[0; 6]);
        assert_eq!(
            ftex::dds::read_layout(&dds).unwrap().pixel,
            ftex::dds::DdsPixel::Uncompressed {
                bit_count: 24,
                r_mask: 0xff0000,
                g_mask: 0xff00,
                b_mask: 0xff,
                a_mask: 0xff000000,
            }
        );
        assert!(matches!(
            decode(&dds, SourceFormat::Dds),
            Err(ConvertError::Unsupported("pixel masks"))
        ));
    }

    #[test]
    fn an_opaque_bc7_source_uses_the_opaque_preset() {
        // A fully opaque chain takes the opaque settings branch: the
        // emitted level-0 blocks are the CPU encoder's opaque_basic
        // output for the same pixels.
        use block_compression::encode::compress_rgba8;
        use block_compression::{BC7Settings, CompressionVariant};
        let decoded = decode(PNG_OPAQUE, SourceFormat::Png).unwrap();
        let round_dds = encode_dds(&decoded, BlockCodec::Bc7).unwrap();
        let layout = ftex::dds::read_layout(&round_dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc7)
        );
        let variant = CompressionVariant::BC7(BC7Settings::opaque_basic());
        let mut expected = vec![0u8; variant.blocks_byte_size(32, 16)];
        compress_rgba8(variant, &decoded.mips[0], &mut expected, 32, 16, 128);
        assert_eq!(
            &round_dds[layout.data_offset..layout.data_offset + expected.len()],
            expected.as_slice()
        );
        // The two presets coincide on fully-opaque pixels (the alpha
        // channel is constant, so the extra channel weight and mode-7
        // tuning never change the argmin — checked exhaustively over
        // random opaque inputs); the translucent source is what makes the
        // branch distinguishable: its emitted blocks are the alpha_basic
        // encode, which the presets differ on.
        let translucent = decode(PNG, SourceFormat::Png).unwrap();
        let alpha_variant = CompressionVariant::BC7(BC7Settings::alpha_basic());
        let mut alpha = vec![0u8; alpha_variant.blocks_byte_size(32, 16)];
        compress_rgba8(alpha_variant, &translucent.mips[0], &mut alpha, 32, 16, 128);
        let mut opaque_blocks = vec![0u8; variant.blocks_byte_size(32, 16)];
        compress_rgba8(
            variant,
            &translucent.mips[0],
            &mut opaque_blocks,
            32,
            16,
            128,
        );
        assert_ne!(alpha, opaque_blocks);
        let translucent_round = encode_dds(&translucent, BlockCodec::Bc7).unwrap();
        let translucent_layout = ftex::dds::read_layout(&translucent_round).unwrap();
        assert_eq!(
            &translucent_round
                [translucent_layout.data_offset..translucent_layout.data_offset + alpha.len()],
            alpha.as_slice()
        );
        // The decoded quality, judged like the translucent BC7 encode.
        let generated: Vec<Vec<u8>> = std::iter::once(decoded.mips[0].clone())
            .chain(
                mips::generate(32, 16, &decoded.mips[0])
                    .into_iter()
                    .map(|mip| mip.pixels.into_owned()),
            )
            .collect();
        let round = decode(&round_dds, SourceFormat::Dds).unwrap();
        assert_not_worse_than_reference(
            "png opaque -> pes21 bc7",
            &generated,
            &round.mips,
            &generated,
            &mips_of(BC1_D),
        );
    }

    #[test]
    fn bmp_webp_and_jpeg_sources_decode_to_the_expected_rgba() {
        // Pillow's decode is the expected RGBA8 (fixture README, sixth
        // round): BMP and lossless WebP exactly; JPEG within the IDCT
        // rounding between decoders.
        const BMP: &[u8] = include_bytes!("../tests/fixtures/source_opaque.bmp");
        const BMP_RGBA: &[u8] = include_bytes!("../tests/fixtures/source_opaque.bmp.rgba");
        const WEBP: &[u8] = include_bytes!("../tests/fixtures/source.webp");
        const WEBP_RGBA: &[u8] = include_bytes!("../tests/fixtures/source.webp.rgba");
        const JPG: &[u8] = include_bytes!("../tests/fixtures/source_opaque.jpg");
        const JPG_RGBA: &[u8] = include_bytes!("../tests/fixtures/source_opaque.jpg.rgba");
        for (bytes, format, expected) in [
            (BMP, SourceFormat::Bmp, BMP_RGBA),
            (WEBP, SourceFormat::WebP, WEBP_RGBA),
        ] {
            let decoded = decode(bytes, format).unwrap();
            assert_eq!((decoded.width, decoded.height), (32, 16));
            assert_eq!(decoded.mips.len(), 1);
            assert!(!decoded.authored_mips);
            assert_eq!(decoded.mips[0].as_slice(), expected);
        }
        let decoded = decode(JPG, SourceFormat::Jpeg).unwrap();
        assert_eq!((decoded.width, decoded.height), (32, 16));
        assert_eq!(decoded.mips.len(), 1);
        assert!(!decoded.authored_mips);
        let max_diff = decoded.mips[0]
            .iter()
            .zip(JPG_RGBA)
            .map(|(ours, theirs)| ours.abs_diff(*theirs))
            .max()
            .unwrap_or(0);
        assert!(max_diff <= 2, "jpg max channel diff {max_diff}");
    }

    #[test]
    fn a_lossless_jxl_decodes_as_its_png_does() {
        // The bare codestream, then the same codestream in the ISOBMFF container.
        for jxl in [JXL_LOSSLESS, JXL_CONTAINER] {
            assert_eq!(
                decode(jxl, SourceFormat::Jxl).unwrap(),
                decode(PNG, SourceFormat::Png).unwrap()
            );
        }
    }

    #[test]
    fn a_lossy_jxl_decodes_within_two_levels_of_libjxl_s_decode() {
        // Two decoders of a lossy file may round differently (fixture README).
        const LIBJXL_RGBA: &[u8] = include_bytes!("../tests/fixtures/source.lossy.jxl.rgba");
        let decoded = decode(JXL_LOSSY, SourceFormat::Jxl).unwrap();
        assert_eq!((decoded.width, decoded.height), (32, 16));
        assert_eq!(decoded.mips.len(), 1);
        assert!(!decoded.authored_mips);
        assert_eq!(decoded.mips[0].len(), LIBJXL_RGBA.len());
        let (index, ours, theirs) = decoded.mips[0]
            .iter()
            .zip(LIBJXL_RGBA)
            .enumerate()
            .map(|(index, (ours, theirs))| (index, *ours, *theirs))
            .max_by_key(|(_, ours, theirs)| ours.abs_diff(*theirs))
            .unwrap();
        assert!(
            ours.abs_diff(theirs) <= 2,
            "pixel {} channel {}: {ours} against libjxl's {theirs}",
            index / 4,
            index % 4
        );
    }

    #[test]
    fn the_mip_count_comes_from_the_field_not_the_caps_bit() {
        // DirectXTex reads mipmap_count alone; a file whose DDSCAPS_MIPMAP
        // bit is cleared still carries its levels.
        let mut dds = BC3.to_vec();
        let caps = u32::from_le_bytes(dds[108..112].try_into().unwrap());
        dds[108..112].copy_from_slice(&(caps & !0x400000).to_le_bytes());
        let ours = decode(&dds, SourceFormat::Dds).unwrap();
        let expected = decode(BC3_D, SourceFormat::Dds).unwrap();
        assert_eq!(ours.mips.len(), expected.mips.len());
        assert_eq!(ours.mips.len(), 6);
    }

    #[test]
    fn a_block_texture_past_the_decoders_4gib_limit_is_refused() {
        // DXT1 at 32768x32772: the padded RGBA mip exceeds the u32 offsets
        // block_compression's decoder writes with. The header alone (no
        // data) reaches the refusal before any slicing.
        let mut dds = uncompressed_dds(32768, 32772, 0, 0, [0; 4], 1);
        dds[80..84].copy_from_slice(&0x4u32.to_le_bytes()); // DDPF_FOURCC
        dds[84..88].copy_from_slice(b"DXT1");
        assert!(matches!(
            decode(&dds, SourceFormat::Dds),
            Err(ConvertError::Unsupported(
                "block texture past the decoder's 4 GiB limit"
            ))
        ));
    }

    #[test]
    fn an_rgb_header_ignores_an_undeclared_alpha_mask() {
        // DDPF_RGB without DDPF_ALPHAPIXELS: the fourth mask does not
        // exist as far as the decode is concerned (DirectXTex's RGB
        // match), so the pixel is opaque whatever the mask says.
        let mut dds = uncompressed_dds(2, 1, 8, 32, [0xff0000, 0xff00, 0xff, 0xff000000], 1);
        dds.extend_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0]);
        assert_eq!(
            decode(&dds, SourceFormat::Dds).unwrap().mips[0],
            [255, 0, 0, 255, 0, 255, 0, 255]
        );
    }

    #[test]
    fn compatible_blocks_pass_through() {
        // BC7 -> PES 21: FTEX carrying the untouched blocks.
        let decoded = decode(BC7, SourceFormat::Dds).unwrap();
        let ftex = convert(
            &decoded,
            Target {
                version: PesVersion::Pes21,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let round = ftex::ftex_to_dds(&ftex).unwrap();
        let layout = ftex::dds::read_layout(&round).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc7)
        );
        assert_eq!((layout.width, layout.height, layout.mipmaps), (32, 16, 6));
        let source_layout = ftex::dds::read_layout(BC7).unwrap();
        assert_eq!(
            &round[layout.data_offset..],
            &BC7[source_layout.data_offset..]
        );

        // BC3 -> PES 17: legacy-header DDS, identical data.
        let decoded = decode(BC3, SourceFormat::Dds).unwrap();
        let dds = convert(
            &decoded,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        assert_eq!(&dds[layout.data_offset..], &BC3[128..]);

        // BC5 -> PES 21 Normal role: still a passthrough.
        let decoded = decode(BC5, SourceFormat::Dds).unwrap();
        let ftex = convert(
            &decoded,
            Target {
                version: PesVersion::Pes21,
                role: TextureRole::Normal,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let round = ftex::ftex_to_dds(&ftex).unwrap();
        let layout = ftex::dds::read_layout(&round).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc5)
        );
        let source_layout = ftex::dds::read_layout(BC5).unwrap();
        assert_eq!(
            &round[layout.data_offset..],
            &BC5[source_layout.data_offset..]
        );
    }

    #[test]
    fn bc7_transcodes_to_bc3_for_older_versions() {
        let decoded = decode(BC7, SourceFormat::Dds).unwrap();
        let truth = mips_of(BC7_D);
        let source = mips_of(RGBA8_D);
        let reference_bc3 = mips_of(BC3_D);

        let dds = convert(
            &decoded,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        assert_eq!(layout.mipmaps, 6);
        assert_not_worse_than_reference(
            "pes17 bc3",
            &truth,
            &mips_of(&dds),
            &source,
            &reference_bc3,
        );

        let ftex = convert(
            &decoded,
            Target {
                version: PesVersion::Pes18,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        assert_eq!(&ftex[..4], b"FTEX");
        let round = decode(&ftex, SourceFormat::Ftex).unwrap();
        assert_not_worse_than_reference("pes18 bc3", &truth, &round.mips, &source, &reference_bc3);
    }

    /// The pixel format an emitted container declares, FTEX or DDS.
    fn emitted_format(output: &[u8]) -> ftex::PixelFormat {
        if output.starts_with(b"FTEX") {
            return ftex::info(output).unwrap().format;
        }
        match ftex::dds::read_layout(output).unwrap().pixel {
            ftex::dds::DdsPixel::Format(format) => format,
            ftex::dds::DdsPixel::Uncompressed { .. } => {
                panic!("a layout with no FTEX format")
            }
        }
    }

    /// An A8R8G8B8 DDS (the canonical legacy header) of `rgba_mips`, each
    /// level's RGBA8 pixels stored B,G,R,A per pixel.
    fn argb8_dds(width: u32, height: u32, rgba_mips: &[Vec<u8>]) -> Vec<u8> {
        let mut dds = ftex::dds::header_bytes(
            ftex::PixelFormat::Argb8,
            width,
            height,
            rgba_mips.len() as u32,
        );
        for mip in rgba_mips {
            for pixel in mip.as_chunks::<4>().0 {
                dds.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            }
        }
        dds
    }

    #[test]
    fn an_encode_is_bc1_or_bc3_on_every_version_never_bc7() {
        let alpha = decode(PNG, SourceFormat::Png).unwrap();
        let opaque = decode(PNG_OPAQUE, SourceFormat::Png).unwrap();
        for version in PesVersion::ALL {
            for role in [TextureRole::Color, TextureRole::Normal] {
                for (decoded, opaque_source) in [(&alpha, false), (&opaque, true)] {
                    let output = convert(
                        decoded,
                        Target {
                            version,
                            role,
                            keep_uncompressed: true,
                        },
                    )
                    .unwrap();
                    // The cases the codec pick weighs: alpha makes BC3, opaque
                    // BC1; a normal-role source is BC3 (DXT5nm) whatever its
                    // alpha. No version encodes BC7.
                    let expected = if role == TextureRole::Color && opaque_source {
                        ftex::PixelFormat::Bc1
                    } else {
                        ftex::PixelFormat::Bc3
                    };
                    assert_eq!(
                        emitted_format(&output),
                        expected,
                        "{version} {role:?} opaque {opaque_source}"
                    );
                }
            }
        }
    }

    #[test]
    fn an_uncompressed_source_keeps_its_layout_when_the_target_asks() {
        // 8x4 with two authored levels, alpha 0 in some pixels: every byte
        // different so a swapped or dropped channel shows.
        let level = |pixels: u32| {
            (0..pixels)
                .flat_map(|i| [i as u8, (i * 7) as u8, (255 - i) as u8, (i % 3) as u8])
                .collect::<Vec<u8>>()
        };
        let source = argb8_dds(8, 4, &[level(32), level(8)]);
        let decoded = decode(&source, SourceFormat::Dds).unwrap();
        assert!(decoded.uncompressed);
        let target = |version, role, keep_uncompressed| Target {
            version,
            role,
            keep_uncompressed,
        };

        // PES 21: an FTEX of format 0 (Argb8) whose decode is the source's,
        // alpha-0 pixels and the second level included.
        let ftex = convert(
            &decoded,
            target(PesVersion::Pes21, TextureRole::Color, true),
        )
        .unwrap();
        assert_eq!(ftex::info(&ftex).unwrap().format, ftex::PixelFormat::Argb8);
        assert_eq!(
            decode(&ftex, SourceFormat::Ftex).unwrap().mips,
            decoded.mips
        );

        // PES 17: a DDS in the same A8R8G8B8 layout, the same decode.
        let dds = convert(
            &decoded,
            target(PesVersion::Pes17, TextureRole::Color, true),
        )
        .unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Argb8)
        );
        assert_eq!(decode(&dds, SourceFormat::Dds).unwrap().mips, decoded.mips);

        // `keep_uncompressed` false encodes: alpha pixels make it BC3; the R8
        // fixture below, whose decode is fully opaque, makes BC1.
        let encoded = convert(
            &decoded,
            target(PesVersion::Pes21, TextureRole::Color, false),
        )
        .unwrap();
        assert_eq!(emitted_format(&encoded), ftex::PixelFormat::Bc3);

        // A normal-role source encodes either way: BC3 in the DXT5nm layout.
        for keep_uncompressed in [true, false] {
            let encoded = convert(
                &decoded,
                target(PesVersion::Pes21, TextureRole::Normal, keep_uncompressed),
            )
            .unwrap();
            assert_eq!(
                emitted_format(&encoded),
                ftex::PixelFormat::Bc3,
                "keep_uncompressed {keep_uncompressed}"
            );
        }

        // An R8 source kept comes back as A8R8G8B8: the same decode; asked to
        // encode, its opaque pixels are BC1.
        let r8 = decode(R8_DDS, SourceFormat::Dds).unwrap();
        assert!(r8.uncompressed);
        let ftex = convert(&r8, target(PesVersion::Pes21, TextureRole::Color, true)).unwrap();
        assert_eq!(ftex::info(&ftex).unwrap().format, ftex::PixelFormat::Argb8);
        assert_eq!(decode(&ftex, SourceFormat::Ftex).unwrap().mips, r8.mips);
        let encoded = convert(&r8, target(PesVersion::Pes21, TextureRole::Color, false)).unwrap();
        assert_eq!(emitted_format(&encoded), ftex::PixelFormat::Bc1);
    }

    #[test]
    fn raster_sources_get_full_mip_chain_and_codec_rules() {
        let decoded = decode(PNG, SourceFormat::Png).unwrap();
        assert_eq!((decoded.width, decoded.height), (32, 16));
        assert_eq!(decoded.mips.len(), 1);
        assert!(decoded.blocks.is_none());
        let source = mips_of(RGBA8_D);
        assert_eq!(decoded.mips[0], source[0]);

        let dds = convert(
            &decoded,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        assert_eq!(layout.mipmaps, 6);
        let round = decode(&dds, SourceFormat::Dds).unwrap();
        for (level, (width, height)) in [(32u32, 16u32), (16, 8), (8, 4), (4, 2), (2, 1), (1, 1)]
            .iter()
            .enumerate()
        {
            assert_eq!(round.mips[level].len(), (*width * *height * 4) as usize);
        }
        // The whole chain, judged against the chain this crate generated.
        let generated: Vec<Vec<u8>> = std::iter::once(decoded.mips[0].clone())
            .chain(
                mips::generate(32, 16, &decoded.mips[0])
                    .into_iter()
                    .map(|mip| mip.pixels.into_owned()),
            )
            .collect();
        assert_not_worse_than_reference(
            "png -> pes17 bc3",
            &generated,
            &round.mips,
            &source,
            &mips_of(BC3_D),
        );

        // Fully opaque raster -> BC1 on pre-Fox.
        let opaque = decode(PNG_OPAQUE, SourceFormat::Png).unwrap();
        let dds = convert(
            &opaque,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc1)
        );
        let round = decode(&dds, SourceFormat::Dds).unwrap();
        assert_not_worse_than_reference(
            "png opaque -> pes17 bc1",
            &opaque.mips,
            &round.mips[..1],
            &decode(PNG_OPAQUE, SourceFormat::Png).unwrap().mips,
            &mips_of(BC1_D)[..1],
        );

        // PES 21 color -> BC3 FTEX, whole chain.
        let ftex = convert(
            &decoded,
            Target {
                version: PesVersion::Pes21,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        assert_eq!(&ftex[..4], b"FTEX");
        assert_eq!(ftex::info(&ftex).unwrap().format, ftex::PixelFormat::Bc3);
        let round = decode(&ftex, SourceFormat::Ftex).unwrap();
        assert_not_worse_than_reference(
            "png -> pes21 bc3",
            &generated,
            &round.mips,
            &source,
            &mips_of(BC3_D),
        );
    }

    #[test]
    fn normal_maps_encode_in_the_dxt5nm_layout() {
        let decoded = decode(BC5, SourceFormat::Dds).unwrap();
        let bc5 = mips_of(BC5_D);
        // The reference: the pre-Fox layout encoded by the reference encoder
        // from the plain source (a `gggr` swizzle), decoded.
        let reference = mips_of(NM_PREFOX_D);
        let reference_truth = dxt5nm(&mips_of(RGBA8_D), false);

        // Pre-Fox: the colour block is a grey Y (R = G = B = Y), X in alpha.
        let dds = convert(
            &decoded,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Normal,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        assert_not_worse_than_reference(
            "pes17 dxt5nm",
            &dxt5nm(&bc5, false),
            &mips_of(&dds),
            &reference_truth,
            &reference,
        );

        // Fox: R = 255, B = 0 exactly (constant channels quantize exactly);
        // G and A judged against the same reference, relaid in the Fox
        // layout.
        let ftex = convert(
            &decoded,
            Target {
                version: PesVersion::Pes18,
                role: TextureRole::Normal,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let ours = decode(&ftex, SourceFormat::Ftex).unwrap().mips;
        for px in ours.iter().flat_map(|mip| mip.as_chunks::<4>().0.iter()) {
            assert_eq!((px[0], px[2]), (255, 0), "fox R/B");
        }
        assert_not_worse_than_reference(
            "pes18 dxt5nm",
            &dxt5nm(&bc5, true),
            &ours,
            &dxt5nm(&mips_of(RGBA8_D), true),
            &dxt5nm(&plain_from_prefox(&reference), true),
        );

        // A raster normal on PES 21 is BC3, never BC7.
        let decoded = decode(PNG, SourceFormat::Png).unwrap();
        let ftex = convert(
            &decoded,
            Target {
                version: PesVersion::Pes21,
                role: TextureRole::Normal,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let round = ftex::ftex_to_dds(&ftex).unwrap();
        let layout = ftex::dds::read_layout(&round).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
    }

    #[test]
    fn authored_mip_count_is_kept() {
        // rgba8.dds cut to a single level: mip count 1, the mipmap caps bit
        // cleared, only level-0 data kept (DX10 header, 4 bytes per pixel).
        let mut single = RGBA8[..148 + 32 * 16 * 4].to_vec();
        single[28..32].copy_from_slice(&1u32.to_le_bytes());
        let caps = u32::from_le_bytes(single[108..112].try_into().unwrap()) & !0x400000;
        single[108..112].copy_from_slice(&caps.to_le_bytes());

        let decoded = decode(&single, SourceFormat::Dds).unwrap();
        assert_eq!(decoded.mips.len(), 1);
        assert!(decoded.authored_mips);

        let ftex = convert(
            &decoded,
            Target {
                version: PesVersion::Pes21,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        assert_eq!(ftex::info(&ftex).unwrap().mipmaps, 1);

        let dds = convert(
            &decoded,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        assert_eq!(ftex::dds::read_layout(&dds).unwrap().mipmaps, 1);
    }

    #[test]
    fn encode_png_writes_every_channel_of_every_texel_as_it_is() {
        // 7 x 5, every byte different (35 texels, 140 bytes), alpha included.
        let (width, height) = (7u32, 5u32);
        let pixels: Vec<u8> = (0..width * height * 4).map(|byte| byte as u8).collect();

        let png = encode_png(&pixels, width, height).unwrap();

        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "a PNG signature");
        let round = decode(&png, SourceFormat::Png).unwrap();
        assert_eq!((round.width, round.height), (width, height));
        assert_eq!(round.mips, [pixels]);
    }

    #[test]
    fn encode_png_refuses_a_zero_size_or_a_buffer_of_the_wrong_length() {
        let pixels = vec![9u8; 4 * 4 * 4];
        for (width, height) in [(0, 4), (4, 0)] {
            assert!(matches!(
                encode_png(&pixels, width, height),
                Err(ConvertError::InvalidDecoded("zero dimension"))
            ));
        }
        for (width, height) in [(4, 3), (4, 5)] {
            assert!(matches!(
                encode_png(&pixels, width, height),
                Err(ConvertError::InvalidDecoded("pixel buffer size"))
            ));
        }
    }

    #[test]
    fn encode_dds_encodes_a_raster_source_as_a_bc3_dds_with_a_full_chain() {
        let decoded = decode(PNG, SourceFormat::Png).unwrap();
        let dds = encode_dds(&decoded, BlockCodec::Bc3).unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        assert_eq!(layout.data_offset, 128, "a legacy DX9 header for BC3");
        assert_eq!(
            (layout.width, layout.height, layout.mipmaps),
            (32, 16, 6),
            "the source's size, the chain down to 1x1"
        );
        let round = decode(&dds, SourceFormat::Dds).unwrap();
        for (level, (width, height)) in [(32u32, 16u32), (16, 8), (8, 4), (4, 2), (2, 1), (1, 1)]
            .iter()
            .enumerate()
        {
            assert_eq!(round.mips[level].len(), (*width * *height * 4) as usize);
        }
        let generated: Vec<Vec<u8>> = std::iter::once(decoded.mips[0].clone())
            .chain(
                mips::generate(32, 16, &decoded.mips[0])
                    .into_iter()
                    .map(|mip| mip.pixels.into_owned()),
            )
            .collect();
        assert_not_worse_than_reference(
            "png -> bc3 dds",
            &generated,
            &round.mips,
            &mips_of(RGBA8_D),
            &mips_of(BC3_D),
        );
        // The same source encoded for a pre-Fox target is BC3 too, so the
        // one difference from `convert` is that the codec is named: the two
        // paths agree byte for byte.
        let converted = convert(
            &decoded,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        assert_eq!(dds, converted);
    }

    #[test]
    fn encode_dds_keeps_blocks_already_in_the_codec_and_reencodes_the_rest() {
        // BC3 blocks under BC3: kept, behind the legacy header.
        let dds = encode_dds(&decode(BC3, SourceFormat::Dds).unwrap(), BlockCodec::Bc3).unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        assert_eq!(&dds[layout.data_offset..], &BC3[128..]);

        // BC7 blocks under BC7: kept, behind a DX10 header.
        let bc7 = decode(BC7, SourceFormat::Dds).unwrap();
        let dds = encode_dds(&bc7, BlockCodec::Bc7).unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc7)
        );
        assert_eq!(layout.data_offset, 148);
        let source_layout = ftex::dds::read_layout(BC7).unwrap();
        assert_eq!(
            &dds[layout.data_offset..],
            &BC7[source_layout.data_offset..]
        );

        // BC7 blocks under BC3: re-encoded, the authored mip count kept.
        let dds = encode_dds(&bc7, BlockCodec::Bc3).unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        assert_eq!(layout.mipmaps, 6);
        assert_not_worse_than_reference(
            "bc7 -> bc3 dds",
            &mips_of(BC7_D),
            &mips_of(&dds),
            &mips_of(RGBA8_D),
            &mips_of(BC3_D),
        );
    }

    #[test]
    fn encode_dds_refuses_the_codecs_this_crate_does_not_encode() {
        let decoded = decode(BC5, SourceFormat::Dds).unwrap();
        for codec in [BlockCodec::Bc2, BlockCodec::Bc4, BlockCodec::Bc5] {
            assert!(
                matches!(
                    encode_dds(&decoded, codec),
                    Err(ConvertError::Unsupported("codec not encoded"))
                ),
                "{codec:?}"
            );
        }
        // The same checks as `convert` on a caller-built value.
        let mut zero = decoded;
        zero.width = 0;
        assert!(matches!(
            encode_dds(&zero, BlockCodec::Bc3),
            Err(ConvertError::InvalidDecoded("zero dimension"))
        ));
    }

    #[test]
    fn normal_role_reencodes_color_layouts() {
        // A BC7 source is a color layout: the normal role re-encodes it to
        // the Fox DXT5nm layout instead of keeping the blocks.
        let ftex = convert(
            &decode(BC7, SourceFormat::Dds).unwrap(),
            Target {
                version: PesVersion::Pes21,
                role: TextureRole::Normal,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let round = ftex::ftex_to_dds(&ftex).unwrap();
        let layout = ftex::dds::read_layout(&round).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        let ours = decode(&round, SourceFormat::Dds).unwrap();
        for px in ours
            .mips
            .iter()
            .flat_map(|mip| mip.as_chunks::<4>().0.iter())
        {
            assert_eq!((px[0], px[2]), (255, 0), "fox R/B");
        }

        // A BC1 source on a pre-Fox target: BC3 with a grey color block.
        let dds = convert(
            &decode(BC1, SourceFormat::Dds).unwrap(),
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Normal,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
        let ours = decode(&dds, SourceFormat::Dds).unwrap();
        // The color block is a grey Y; 565 quantization gives G a sixth bit
        // R and B do not have, so R == B exactly and G within half a step.
        for px in ours
            .mips
            .iter()
            .flat_map(|mip| mip.as_chunks::<4>().0.iter())
        {
            assert_eq!(px[0], px[2], "grey color block R/B");
            assert!(px[1].abs_diff(px[0]) <= 4, "grey color block G");
        }

        // A BC3 source still passes through on a normal role.
        let dds = convert(
            &decode(BC3, SourceFormat::Dds).unwrap(),
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Normal,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let layout = ftex::dds::read_layout(&dds).unwrap();
        assert_eq!(&dds[layout.data_offset..], &BC3[128..]);
    }

    /// A minimal uncompressed DDS: legacy header declaring `bit_count`-bit
    /// pixels with `masks` and a row pitch of `pitch` bytes, then space for
    /// `mipmaps` levels of data appended by the caller.
    fn uncompressed_dds(
        width: u32,
        height: u32,
        pitch: u32,
        bit_count: u32,
        masks: [u32; 4],
        mipmaps: u32,
    ) -> Vec<u8> {
        let mut dds = Vec::new();
        dds.extend_from_slice(b"DDS ");
        dds.extend_from_slice(&124u32.to_le_bytes());
        dds.extend_from_slice(&(0x1u32 | 0x2 | 0x4 | 0x8 | 0x1000).to_le_bytes());
        dds.extend_from_slice(&height.to_le_bytes());
        dds.extend_from_slice(&width.to_le_bytes());
        dds.extend_from_slice(&pitch.to_le_bytes());
        dds.extend_from_slice(&0u32.to_le_bytes()); // depth
        dds.extend_from_slice(&mipmaps.to_le_bytes());
        dds.extend_from_slice(&[0u8; 44]);
        dds.extend_from_slice(&32u32.to_le_bytes()); // pixel format size
        dds.extend_from_slice(&0x40u32.to_le_bytes()); // DDPF_RGB
        dds.extend_from_slice(&[0u8; 4]); // fourcc
        dds.extend_from_slice(&bit_count.to_le_bytes());
        for mask in masks {
            dds.extend_from_slice(&mask.to_le_bytes());
        }
        let caps1 = 0x1000u32 | if mipmaps > 1 { 0x8 | 0x400000 } else { 0 };
        dds.extend_from_slice(&caps1.to_le_bytes());
        dds.extend_from_slice(&[0u8; 16]);
        dds
    }

    #[test]
    fn padded_rows_decode() {
        // A 2x2 24-bit BGR DDS whose rows are padded to 8 bytes.
        let header = |pitch: u32| uncompressed_dds(2, 2, pitch, 24, [0xff0000, 0xff00, 0xff, 0], 1);
        let expected: Vec<u8> = [
            [255, 0, 0, 255], // stored BGR 0,0,255
            [0, 255, 0, 255], // stored BGR 0,255,0
            [0, 0, 255, 255], // stored BGR 255,0,0
            [255, 255, 255, 255],
        ]
        .concat();

        // Pitch 8: two pad bytes per row.
        let mut padded = header(8);
        padded.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        padded.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        assert_eq!(
            decode(&padded, SourceFormat::Dds).unwrap().mips,
            [&expected[..]]
        );

        // Pitch 6 equals the tight row: no padding, same pixels.
        let mut tight = header(6);
        tight.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
        tight.extend_from_slice(&[255, 0, 0, 255, 255, 255]);
        assert_eq!(
            decode(&tight, SourceFormat::Dds).unwrap().mips,
            [&expected[..]]
        );
    }

    #[test]
    fn dword_padded_rows_decode_at_their_own_pitch() {
        // The fixture: level 0 at the declared pitch 20 (tight 18), level 1
        // (3x1) at its DWORD-rounded tight pitch (9 -> 12), matching the
        // reference decoder's -dword read of the same file.
        let ours = decode(BGR24, SourceFormat::Dds).unwrap();
        let expected = decode(BGR24_D, SourceFormat::Dds).unwrap();
        assert_eq!(ours.mips, expected.mips);

        // The same rule on a file whose lower level has two rows, so the
        // row pitch actually moves the second row: 6x4, declared pitch 22
        // (tight 18); level 1 is 3x2 at 12-byte rows.
        let masks = [0xff0000, 0xff00, 0xff, 0];
        let mut dds = uncompressed_dds(6, 4, 22, 24, masks, 2);
        for y in 0..4u8 {
            for x in 0..6u8 {
                let base = y * 64 + x * 8;
                dds.extend_from_slice(&[base, base + 1, base + 2]);
            }
            dds.extend_from_slice(&[0xee; 4]);
        }
        for y in 0..2u8 {
            for x in 0..3u8 {
                let base = 128 + y * 32 + x * 8;
                dds.extend_from_slice(&[base, base + 1, base + 2]);
            }
            dds.extend_from_slice(&[0xdd; 3]);
        }
        let mut level0 = Vec::new();
        for y in 0..4u8 {
            for x in 0..6u8 {
                let base = y * 64 + x * 8;
                level0.extend_from_slice(&[base + 2, base + 1, base, 255]);
            }
        }
        let mut level1 = Vec::new();
        for y in 0..2u8 {
            for x in 0..3u8 {
                let base = 128 + y * 32 + x * 8;
                level1.extend_from_slice(&[base + 2, base + 1, base, 255]);
            }
        }
        assert_eq!(
            decode(&dds, SourceFormat::Dds).unwrap().mips,
            [level0, level1]
        );
    }

    #[test]
    fn an_oversized_uncompressed_declaration_is_truncated_not_a_panic() {
        // 70000x70000 32-bit rows need ~20 GB of stream data the buffer does
        // not hold: the mip lookup is Truncated rather than an arithmetic
        // overflow panic.
        let mut dds =
            uncompressed_dds(70000, 70000, 0, 32, [0xff0000, 0xff00, 0xff, 0xff000000], 1);
        dds.extend_from_slice(&[0u8; 16]);
        assert!(matches!(
            decode(&dds, SourceFormat::Dds),
            Err(ConvertError::Truncated)
        ));
    }

    #[test]
    fn uncompressed_pixel_masks_are_byte_aligned_or_rejected() {
        // A 2x1 24-bit row to decode against.
        let bgr = |masks: [u32; 4]| {
            let mut dds = uncompressed_dds(2, 1, 0, 24, masks, 1);
            dds.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
            dds
        };
        // A mask narrower than eight bits.
        assert!(matches!(
            decode(&bgr([0x0f, 0xff00, 0xff, 0]), SourceFormat::Dds),
            Err(ConvertError::Unsupported("pixel masks"))
        ));
        // A mask not aligned to a byte.
        assert!(matches!(
            decode(&bgr([0xff0, 0xff00, 0xff, 0]), SourceFormat::Dds),
            Err(ConvertError::Unsupported("pixel masks"))
        ));
        // An alpha channel past the pixel's own 24 bits. DDPF_ALPHAPIXELS
        // is what keeps the fourth mask a channel at all; the blue mask is
        // absent so the header is not the canonical Argb8 shape.
        let mut past = bgr([0xff0000, 0xff00, 0, 0xff000000]);
        past[80..84].copy_from_slice(&0x41u32.to_le_bytes());
        assert!(matches!(
            decode(&past, SourceFormat::Dds),
            Err(ConvertError::Unsupported("pixel masks"))
        ));
        // A channel with no mask reads 0 (alpha reads 255); the bytes are
        // B,G,R so 1,2,3 decodes to R=3, B=1.
        let decoded = decode(&bgr([0xff0000, 0, 0xff, 0]), SourceFormat::Dds).unwrap();
        assert_eq!(decoded.mips[0], [3, 0, 1, 255, 6, 0, 4, 255]);
    }

    #[test]
    fn an_all_zero_pixel_mask_set_is_rejected() {
        // A DDPF_RGB header with no channel masks decodes nothing.
        let mut dds = uncompressed_dds(2, 1, 2, 8, [0, 0, 0, 0], 1);
        dds.extend_from_slice(&[3, 5]);
        assert!(matches!(
            decode(&dds, SourceFormat::Dds),
            Err(ConvertError::Unsupported("pixel masks"))
        ));
        // A8 — DDPF_ALPHA with an alpha mask only — still decodes to
        // (0, 0, 0, a).
        let mut a8 = uncompressed_dds(2, 1, 2, 8, [0, 0, 0, 0xff], 1);
        a8[80..84].copy_from_slice(&0x2u32.to_le_bytes()); // DDPF_ALPHA
        a8.extend_from_slice(&[3, 5]);
        assert_eq!(
            decode(&a8, SourceFormat::Dds).unwrap().mips[0],
            [0, 0, 0, 3, 0, 0, 0, 5]
        );
    }

    #[test]
    fn a_u32_max_dimension_declaration_is_unsupported_not_a_panic() {
        // u32::MAX x u32::MAX is a block texture past the decoder's 4 GiB
        // output limit: refused rather than an arithmetic panic.
        let mut dds = Vec::new();
        dds.extend_from_slice(b"DDS ");
        dds.extend_from_slice(&124u32.to_le_bytes());
        dds.extend_from_slice(&(0x1u32 | 0x2 | 0x4 | 0x1000).to_le_bytes());
        dds.extend_from_slice(&u32::MAX.to_le_bytes()); // height
        dds.extend_from_slice(&u32::MAX.to_le_bytes()); // width
        dds.extend_from_slice(&0u32.to_le_bytes()); // linear size
        dds.extend_from_slice(&0u32.to_le_bytes()); // depth
        dds.extend_from_slice(&1u32.to_le_bytes()); // mipmaps
        dds.extend_from_slice(&[0u8; 44]);
        dds.extend_from_slice(&32u32.to_le_bytes()); // pixel format size
        dds.extend_from_slice(&0x4u32.to_le_bytes()); // DDPF_FOURCC
        dds.extend_from_slice(b"DXT5");
        dds.extend_from_slice(&[0u8; 20]); // bit count and masks
        dds.extend_from_slice(&0x1000u32.to_le_bytes()); // caps1: texture
        dds.extend_from_slice(&[0u8; 16]);
        assert!(matches!(
            decode(&dds, SourceFormat::Dds),
            Err(ConvertError::Unsupported(
                "block texture past the decoder's 4 GiB limit"
            ))
        ));
    }

    #[test]
    fn convert_rejects_an_inconsistent_decoded() {
        let target = Target {
            version: PesVersion::Pes17,
            role: TextureRole::Color,
            keep_uncompressed: true,
        };
        let base = || decode(BC3, SourceFormat::Dds).unwrap();
        let refuse = |decoded: &Decoded, what: &str| {
            assert!(
                matches!(
                    convert(decoded, target),
                    Err(ConvertError::InvalidDecoded(_))
                ),
                "{what}"
            );
        };

        let mut zero = base();
        zero.width = 0;
        refuse(&zero, "zero dimension");

        // Zero on one axis alone, with a mip sized for it: only the
        // dimension rule can fire.
        let mut thin = base();
        thin.width = 0;
        thin.mips = vec![vec![0u8; 16 * 4]];
        thin.blocks = None;
        refuse(&thin, "zero on one axis");

        let mut empty = base();
        empty.mips.clear();
        refuse(&empty, "no mips");

        // authored_mips = false means a raster decode, which is one mip.
        let mut raster = base();
        raster.authored_mips = false;
        raster.blocks = None;
        refuse(&raster, "unauthored chain");

        // 32x16 bottoms out at six levels; a seventh has no pixels left.
        let mut deep = base();
        deep.mips.push(deep.mips.last().unwrap().clone());
        deep.blocks = None;
        refuse(&deep, "mip count past the dimensions");

        let mut short = base();
        short.mips[0].truncate(4);
        refuse(&short, "mip size");

        let mut count = base();
        count.blocks.as_mut().unwrap().mips.pop();
        refuse(&count, "block mip count");

        let mut size = base();
        size.blocks.as_mut().unwrap().mips[0].push(0);
        refuse(&size, "block mip size");

        // A mip whose size overflows u64 cannot exist: the declaration is
        // invalid rather than an arithmetic panic.
        let mut huge = base();
        huge.width = 1 << 31;
        huge.height = 1 << 31;
        huge.mips = vec![vec![]];
        huge.blocks = None;
        refuse(&huge, "overflowing mip size");

        // A decoded value always validates.
        convert(&base(), target).unwrap();
    }

    #[test]
    fn a_bc5_source_encodes_dxt5nm_whatever_the_role() {
        // A BC5 source on a pre-Fox target is DXT5nm (BC3 with X in alpha, Y
        // in green) whether the texture is a normal map or not.
        let decoded = decode(BC5, SourceFormat::Dds).unwrap();
        let color = convert(
            &decoded,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Color,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        let normal = convert(
            &decoded,
            Target {
                version: PesVersion::Pes17,
                role: TextureRole::Normal,
                keep_uncompressed: true,
            },
        )
        .unwrap();
        assert_eq!(color, normal);
        let layout = ftex::dds::read_layout(&color).unwrap();
        assert_eq!(
            layout.pixel,
            ftex::dds::DdsPixel::Format(ftex::PixelFormat::Bc3)
        );
    }

    #[test]
    fn mip_generation_rounds_to_nearest() {
        // Channel values 0, 0, 1, 1: a sum of 2 over 4 texels rounds to 1,
        // not 0.
        let mut pixels = vec![0u8; 2 * 2 * 4];
        pixels[2 * 4] = 1;
        pixels[3 * 4] = 1;
        let chain = mips::generate(2, 2, &pixels);
        assert_eq!(chain[0].pixels[0], 1);
        // 1x2 with 40 and 81: (40 + 81 + 1) / 2 = 61.
        let pixels = [40, 0, 0, 255, 81, 0, 0, 255];
        let chain = mips::generate(1, 2, &pixels);
        assert_eq!(chain[0].pixels[0], 61);
    }

    #[test]
    fn mip_generation_averages_the_2x2_box() {
        // 4x2 with channel values that average exactly.
        let mut pixels = vec![0u8; 4 * 2 * 4];
        for (i, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            pixel[0] = i as u8 * 4;
            pixel[1] = 8;
            pixel[3] = 255;
        }
        let chain = mips::generate(4, 2, &pixels);
        assert_eq!(chain.len(), 2);
        assert_eq!((chain[0].width, chain[0].height), (2, 1));
        assert_eq!(chain[0].pixels[0], 10); // (0+4+16+20)/4
        assert_eq!(chain[0].pixels[4], 18); // (8+12+24+28)/4
        assert_eq!((chain[1].width, chain[1].height), (1, 1));
        assert_eq!(chain[1].pixels[0], 14); // (10+18)/2

        // 5x3 -> 2x1 -> 1x1.
        let pixels = vec![128u8; 5 * 3 * 4];
        let chain = mips::generate(5, 3, &pixels);
        let dims: Vec<(u32, u32)> = chain.iter().map(|m| (m.width, m.height)).collect();
        assert_eq!(dims, [(2, 1), (1, 1)]);
    }

    #[test]
    fn mip_chain_keeps_halving_while_either_axis_is_above_1() {
        // 8x2: width halves to 1 across three levels; height is already 1.
        let chain = mips::generate(8, 2, &[0u8; 8 * 2 * 4]);
        let dims: Vec<(u32, u32)> = chain.iter().map(|m| (m.width, m.height)).collect();
        assert_eq!(dims, [(4, 1), (2, 1), (1, 1)]);
        // 2x8 is the mirror image: height keeps halving after width floors at
        // 1, so the loop must still run on `height > 1` alone.
        let chain = mips::generate(2, 8, &[0u8; 2 * 8 * 4]);
        let dims: Vec<(u32, u32)> = chain.iter().map(|m| (m.width, m.height)).collect();
        assert_eq!(dims, [(1, 4), (1, 2), (1, 1)]);
    }

    #[test]
    fn downsample_clamps_the_box_to_the_source_edges() {
        // 1x2: the single output pixel averages the one existing column only
        // (x1 clamps to source_width - 1 = 0). (40 + 80 + 1) / 2 = 60.
        let pixels = [40, 0, 0, 255, 80, 0, 0, 255];
        let chain = mips::generate(1, 2, &pixels);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].pixels.as_ref(), &[60, 0, 0, 255]);

        // 3x1: floor halving emits one pixel covering columns 0-1; the odd
        // third column is not sampled. (100 + 200 + 1) / 2 = 150.
        let pixels = [100, 0, 0, 255, 200, 0, 0, 255, 60, 0, 0, 255];
        let chain = mips::generate(3, 1, &pixels);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].pixels.as_ref(), &[150, 0, 0, 255]);
    }

    #[test]
    fn converter_caches_and_bypasses() {
        let converter = Converter::new();
        let hash = source_hash(BC3);
        let target = Target {
            version: PesVersion::Pes17,
            role: TextureRole::Color,
            keep_uncompressed: true,
        };
        let first = converter
            .convert(hash, BC3, SourceFormat::Dds, target, CachePolicy::Use)
            .unwrap();
        let second = converter
            .convert(hash, BC3, SourceFormat::Dds, target, CachePolicy::Use)
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));

        let other = converter
            .convert(
                hash,
                BC3,
                SourceFormat::Dds,
                Target {
                    version: PesVersion::Pes18,
                    role: TextureRole::Color,
                    keep_uncompressed: true,
                },
                CachePolicy::Use,
            )
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &other));
        assert_eq!(converter.retained_bytes(), first.len() + other.len());

        let retained = converter.retained_bytes();
        converter
            .convert(
                source_hash(BC7),
                BC7,
                SourceFormat::Dds,
                Target {
                    version: PesVersion::Pes21,
                    role: TextureRole::Color,
                    keep_uncompressed: true,
                },
                CachePolicy::Bypass,
            )
            .unwrap();
        assert_eq!(converter.retained_bytes(), retained);

        converter.clear();
        assert_eq!(converter.retained_bytes(), 0);
    }

    #[test]
    fn converter_passes_kept_blocks_through_byte_for_byte() {
        let pes21 = Target {
            version: PesVersion::Pes21,
            role: TextureRole::Color,
            keep_uncompressed: true,
        };
        let pes17 = Target {
            version: PesVersion::Pes17,
            role: TextureRole::Color,
            keep_uncompressed: true,
        };
        let wrapped_bc3 = wezlib::compress(BC3);
        let cases: [(&[u8], SourceFormat, Target, &[u8]); 2] = [
            (BC7, SourceFormat::Dds, pes21, BC7_PES21),
            (&wrapped_bc3, SourceFormat::Dds, pes17, BC3_WESYS_PES17),
        ];
        // An FTEX's zlib streams depend on the deflate backend the build
        // links, which another crate in the same build can switch, so an FTEX
        // is compared through its DDS form: header, mip chain and blocks.
        let as_dds = |bytes: &[u8]| {
            if bytes.starts_with(b"FTEX") {
                ftex::ftex_to_dds(bytes).unwrap()
            } else {
                bytes.to_vec()
            }
        };
        for (bytes, format, target, golden) in cases {
            for policy in [CachePolicy::Use, CachePolicy::Bypass] {
                let output = Converter::new()
                    .convert(source_hash(bytes), bytes, format, target, policy)
                    .unwrap();
                assert_eq!(as_dds(&output), as_dds(golden), "{target:?} {policy:?}");
            }
        }

        // An FTEX source takes the same path through its DDS form.
        let output = Converter::new()
            .convert(
                source_hash(FTEX_BC1),
                FTEX_BC1,
                SourceFormat::Ftex,
                pes21,
                CachePolicy::Bypass,
            )
            .unwrap();
        let via_decode = convert(&decode(FTEX_BC1, SourceFormat::Ftex).unwrap(), pes21).unwrap();
        assert_eq!(&output[..], &via_decode[..]);
    }

    #[test]
    fn converter_decodes_a_source_whose_blocks_do_not_pass_through() {
        // BC7 is not read by PES 17, and a BC1 normal map carries a color
        // layout: both are decoded and encoded again.
        let cases = [
            (
                BC7,
                Target {
                    version: PesVersion::Pes17,
                    role: TextureRole::Color,
                    keep_uncompressed: true,
                },
                ftex::PixelFormat::Bc3,
            ),
            (
                BC1,
                Target {
                    version: PesVersion::Pes17,
                    role: TextureRole::Normal,
                    keep_uncompressed: true,
                },
                ftex::PixelFormat::Bc3,
            ),
        ];
        for (bytes, target, format) in cases {
            let output = Converter::new()
                .convert(
                    source_hash(bytes),
                    bytes,
                    SourceFormat::Dds,
                    target,
                    CachePolicy::Bypass,
                )
                .unwrap();
            let layout = ftex::dds::read_layout(&output).unwrap();
            assert_eq!(layout.pixel, ftex::dds::DdsPixel::Format(format));
            let via_decode = convert(&decode(bytes, SourceFormat::Dds).unwrap(), target).unwrap();
            assert_eq!(&output[..], &via_decode[..], "{target:?}");
        }
    }

    #[test]
    fn wesys_wrapped_dds_decodes_the_same() {
        let wrapped = wezlib::compress(BC3);
        assert_eq!(
            decode(&wrapped, SourceFormat::Dds).unwrap(),
            decode(BC3, SourceFormat::Dds).unwrap()
        );
    }

    #[test]
    fn ftex_source_decodes_like_its_dds_twin() {
        assert_eq!(
            decode(FTEX_BC1, SourceFormat::Ftex).unwrap(),
            decode(DDS_BC1, SourceFormat::Dds).unwrap()
        );
        let decoded = decode(FTEX_BC1, SourceFormat::Ftex).unwrap();
        assert_eq!((decoded.width, decoded.height), (16, 16));
        assert_eq!(decoded.mips.len(), 3);
        assert_eq!(
            decoded.blocks.as_ref().map(|blocks| blocks.codec),
            Some(BlockCodec::Bc1)
        );
    }

    #[test]
    fn bc4_equal_endpoints_decode_the_constant_indices() {
        // a0 == a1 is the six-value mode: indices 6 and 7 are the constants
        // 0 and 255, so texels 6 and 7 of the block are 0 and 255 while the
        // rest are the endpoint value.
        let ours = decode(BC4_EQ, SourceFormat::Dds).unwrap();
        assert_eq!(ours.mips.len(), 1);
        let expected = decode(BC4_EQ_D, SourceFormat::Dds).unwrap();
        assert_eq!(ours.mips[0], expected.mips[0]);
        // Texels 6 and 7 carry indices 6 and 7: the 0 and 255 constants,
        // splatted to all three channels by the one-channel decode.
        assert_eq!(
            &ours.mips[0][6 * 4..8 * 4],
            [0, 0, 0, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn bc5_equal_endpoints_decode_the_constant_indices() {
        // The same construction as the BC4 fixture, hand-built for the
        // two-channel format (DXGI 83): both channel blocks declare equal
        // endpoints, so indices 6 and 7 are the constants 0 and 255.
        let mut dds = Vec::new();
        dds.extend_from_slice(b"DDS ");
        dds.extend_from_slice(&124u32.to_le_bytes());
        dds.extend_from_slice(&(0x1u32 | 0x2 | 0x4 | 0x1000 | 0x80000).to_le_bytes());
        dds.extend_from_slice(&4u32.to_le_bytes()); // height
        dds.extend_from_slice(&4u32.to_le_bytes()); // width
        dds.extend_from_slice(&16u32.to_le_bytes()); // pitch
        dds.extend_from_slice(&0u32.to_le_bytes()); // depth
        dds.extend_from_slice(&1u32.to_le_bytes()); // mipmaps
        dds.extend_from_slice(&[0u8; 44]);
        dds.extend_from_slice(&32u32.to_le_bytes()); // pixel format size
        dds.extend_from_slice(&0x4u32.to_le_bytes()); // DDPF_FOURCC
        dds.extend_from_slice(b"DX10");
        dds.extend_from_slice(&[0u8; 20]); // bit count and masks
        dds.extend_from_slice(&0x1000u32.to_le_bytes()); // caps1: texture
        dds.extend_from_slice(&[0u8; 16]);
        dds.extend_from_slice(&83u32.to_le_bytes()); // DXGI BC5_UNORM
        dds.extend_from_slice(&3u32.to_le_bytes()); // 2D
        dds.extend_from_slice(&0u32.to_le_bytes()); // misc flags
        dds.extend_from_slice(&1u32.to_le_bytes()); // array size
        dds.extend_from_slice(&0u32.to_le_bytes()); // misc flags 2
        // Indices 0..7 then 7..0 across the 16 texels.
        let indices = [0u64, 1, 2, 3, 4, 5, 6, 7, 7, 6, 5, 4, 3, 2, 1, 0]
            .iter()
            .enumerate()
            .fold(0u64, |packed, (texel, index)| {
                packed | (index << (3 * texel))
            });
        let channel = |endpoint: u8| {
            let mut block = [endpoint, endpoint, 0, 0, 0, 0, 0, 0];
            block[2..].copy_from_slice(&indices.to_le_bytes()[..6]);
            block
        };
        dds.extend_from_slice(&channel(100)); // R block
        dds.extend_from_slice(&channel(200)); // G block

        let ours = decode(&dds, SourceFormat::Dds).unwrap();
        // Texel 5 is an interpolated index: equal endpoints decode to the
        // endpoint value. Texels 6 and 7 carry the constant indices.
        assert_eq!(&ours.mips[0][5 * 4..6 * 4], [100, 200, 0, 255]);
        assert_eq!(
            &ours.mips[0][6 * 4..8 * 4],
            [0, 0, 0, 255, 255, 255, 0, 255]
        );
    }

    /// A `width`x`height` 32-bit TGA, every pixel opaque grey.
    fn grey_tga(width: u16, height: u16) -> Vec<u8> {
        let mut tga = vec![
            0, 0, 2, // no id, no colormap, truecolor
            0, 0, 0, 0, 0, // colormap spec
            0, 0, 0, 0, // origin
        ];
        tga.extend_from_slice(&width.to_le_bytes());
        tga.extend_from_slice(&height.to_le_bytes());
        tga.extend_from_slice(&[32, 0x28]); // 32 bits, top-left, eight attribute bits
        for _ in 0..u32::from(width) * u32::from(height) {
            tga.extend_from_slice(&[128, 128, 128, 255]); // BGRA
        }
        tga
    }

    #[test]
    fn probe_agrees_with_decode_and_with_the_converted_level_count() {
        let tga = grey_tga(6, 3);
        for (name, bytes, format) in [
            ("bc7.dds, six levels", BC7, SourceFormat::Dds),
            ("rgba8.dds", RGBA8, SourceFormat::Dds),
            ("bc1 ftex, three levels", FTEX_BC1, SourceFormat::Ftex),
            ("source.png", PNG, SourceFormat::Png),
            ("6x3 tga", tga.as_slice(), SourceFormat::Tga),
            ("source.lossless.jxl", JXL_LOSSLESS, SourceFormat::Jxl),
            (
                "source.lossless.container.jxl",
                JXL_CONTAINER,
                SourceFormat::Jxl,
            ),
            ("source.lossy.jxl", JXL_LOSSY, SourceFormat::Jxl),
        ] {
            let probed = probe(bytes, format).unwrap();
            let decoded = decode(bytes, format).unwrap();
            assert_eq!(
                (probed.width, probed.height),
                (decoded.width, decoded.height),
                "{name}"
            );
            let target = Target {
                version: PesVersion::Pes21,
                role: TextureRole::Color,
                keep_uncompressed: true,
            };
            let converted = ftex::info(&convert(&decoded, target).unwrap()).unwrap();
            assert_eq!(probed.mipmaps, u32::from(converted.mipmaps), "{name}");
        }
        // The generated chains, spelled out: 6x3, 3x1, 1x1, and 32x16 down to 1x1.
        assert_eq!(probe(&tga, SourceFormat::Tga).unwrap().mipmaps, 3);
        for jxl in [JXL_LOSSLESS, JXL_CONTAINER, JXL_LOSSY] {
            assert_eq!(
                probe(jxl, SourceFormat::Jxl).unwrap(),
                Probe {
                    width: 32,
                    height: 16,
                    mipmaps: 6
                }
            );
        }
    }

    #[test]
    fn a_wesys_wrapped_dds_probes_like_the_unwrapped_one() {
        let wrapped = wezlib::compress(BC7);
        assert_eq!(
            probe(&wrapped, SourceFormat::Dds).unwrap(),
            probe(BC7, SourceFormat::Dds).unwrap()
        );
    }

    #[test]
    fn a_cut_header_does_not_probe() {
        let tga = grey_tga(6, 3);
        for (name, cut, format) in [
            ("dds", &BC7[..100], SourceFormat::Dds),
            ("ftex", &FTEX_BC1[..32], SourceFormat::Ftex),
            ("png", &PNG[..20], SourceFormat::Png),
            ("tga", &tga[..10], SourceFormat::Tga),
            // The signature and its first two header bytes: this file's header is five.
            ("jxl", &JXL_LOSSLESS[..4], SourceFormat::Jxl),
        ] {
            assert!(probe(cut, format).is_err(), "{name}");
        }
    }

    #[test]
    fn accepted_extensions_resolve() {
        for (extension, format) in [
            ("dds", SourceFormat::Dds),
            ("ftex", SourceFormat::Ftex),
            ("png", SourceFormat::Png),
            ("jpg", SourceFormat::Jpeg),
            ("jpeg", SourceFormat::Jpeg),
            ("bmp", SourceFormat::Bmp),
            ("webp", SourceFormat::WebP),
            ("tga", SourceFormat::Tga),
            ("tif", SourceFormat::Tiff),
            ("tiff", SourceFormat::Tiff),
            ("jxl", SourceFormat::Jxl),
        ] {
            assert_eq!(
                SourceFormat::from_extension(extension),
                Some(format),
                "{extension}"
            );
            assert_eq!(
                SourceFormat::from_extension(&extension.to_uppercase()),
                Some(format),
                "{extension} upper-case"
            );
        }
        assert_eq!(SourceFormat::from_extension("gif"), None);
        assert_eq!(SourceFormat::from_extension(""), None);
    }

    #[test]
    fn malformed_sources_error() {
        assert!(matches!(
            decode(&BC3[..100], SourceFormat::Dds),
            Err(ConvertError::Ftex(ftex::FtexError::Truncated))
        ));
        assert!(matches!(
            decode(CUBE, SourceFormat::Dds),
            Err(ConvertError::Ftex(ftex::FtexError::UnsupportedDds(_)))
        ));
        assert!(matches!(
            decode(b"not a png", SourceFormat::Png),
            Err(ConvertError::Image(_))
        ));
        assert!(matches!(
            decode(&JXL_LOSSLESS[..20], SourceFormat::Jxl),
            Err(ConvertError::Truncated)
        ));
    }
}
