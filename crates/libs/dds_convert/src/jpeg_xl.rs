//! JPEG XL files through Google's `jxl` decoder, called directly rather than
//! registered with the `image` crate: that registration is process-wide, so
//! every other caller of `image` would inherit it. Sources -> [`Decoded`]:
//! one mip of straight-alpha RGBA8, as a PNG decodes, on the calling thread.

use image::ImageError;
use image::error::{DecodingError, ImageFormatHint};
use jxl::api::{AlphaMode, Event, JxlDecoder, JxlDecoderOptions, JxlOutputBuffer, JxlPixelFormat};

use crate::{ConvertError, Decoded, alloc_len};

/// A decoder whose output is straight alpha whatever the file stores.
fn new_decoder() -> JxlDecoder {
    let mut options = JxlDecoderOptions::default();
    options.alpha_mode = AlphaMode::Unpremultiplied;
    JxlDecoder::new(options)
}

/// A `jxl` error as the error of an undecodable raster source.
fn decode_error(error: jxl::error::Error) -> ConvertError {
    ConvertError::Image(ImageError::Decoding(DecodingError::new(
        ImageFormatHint::Name("JPEG XL".to_owned()),
        error,
    )))
}

/// Reads the file's header from `input` up to its basic info: the width and
/// height, oriented as the file says it is shown.
fn read_header(decoder: &mut JxlDecoder, input: &mut &[u8]) -> Result<(u32, u32), ConvertError> {
    // The only other event before the basic info is a request for more
    // input, which a whole file in memory meets only when it is cut short.
    if decoder.process(input, None, None).map_err(decode_error)? != Event::BasicInfo {
        return Err(ConvertError::Truncated);
    }
    let (width, height) = decoder
        .basic_info()
        .expect("the basic info event leaves the basic info read")
        .size;
    let side = |side: usize| {
        u32::try_from(side).map_err(|_| ConvertError::Unsupported("jpeg xl side past 2^32"))
    };
    Ok((side(width)?, side(height)?))
}

/// The width and height a JPEG XL file's header declares, read without
/// decoding a pixel.
pub(crate) fn dimensions(bytes: &[u8]) -> Result<(u32, u32), ConvertError> {
    let mut input = bytes;
    read_header(&mut new_decoder(), &mut input)
}

/// Decodes a JPEG XL file's first frame to a single-mip RGBA8 texture. The
/// decoder writes RGBA8 whatever the file stores: R = G = B for a greyscale
/// file, alpha 255 for one without alpha, 8-bit samples as stored. Samples
/// of more than 8 bits, and a lossy (XYB) file's, are rounded to 8 bits
/// after an ordered dither of under half a level, as libjxl renders 8 bits,
/// so one can land a level away from a plain rounding; 0 and full scale stay
/// exact. A lossy file is rendered to the colour space its header declares,
/// or to sRGB when that is an ICC profile.
pub(crate) fn decode(bytes: &[u8]) -> Result<Decoded, ConvertError> {
    let mut input = bytes;
    let mut decoder = new_decoder();
    let (width, height) = read_header(&mut decoder, &mut input)?;
    let extra_channels = decoder
        .basic_info()
        .expect("read_header leaves the basic info read")
        .extra_channels
        .len();
    decoder
        .set_pixel_format(JxlPixelFormat::rgba8(extra_channels))
        .map_err(decode_error)?;
    let row_bytes = alloc_len(u64::from(width) * 4)?;
    let mut pixels = vec![0; alloc_len(u64::from(width) * u64::from(height) * 4)?];
    let mut output = JxlOutputBuffer::new(&mut pixels, height as usize, row_bytes);
    loop {
        match decoder
            .process(&mut input, Some(std::slice::from_mut(&mut output)), None)
            .map_err(decode_error)?
        {
            Event::FrameHeader => {}
            Event::FrameComplete { .. } => break,
            // A request for more input (the file ends inside the frame), or the
            // file's end with no frame shown, which no encoder writes: both read
            // as a file cut short.
            _ => return Err(ConvertError::Truncated),
        }
    }
    Ok(Decoded {
        width,
        height,
        mips: vec![pixels],
        blocks: None,
        authored_mips: false,
        uncompressed: false,
    })
}
