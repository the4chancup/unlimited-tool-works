//! The team's logo (`spec/team_compiler/pipeline.md` "4. Per-export non-model steps", Logo): the
//! game's three square PNGs, 512, 256 and 128 pixels, made from the export's root `logo*` image
//! of any size and format and, for the 128-pixel one, its `logo_small*` image when there is one.
//! A non-square image is made square by its file's mode: `fit` (the default), `crop` or
//! `stretch`.

use std::sync::Arc;

use aesthetics_export::{LogoFile, LogoFiles, LogoFit};
use dds_convert::{ConvertError, decode, encode_png, resize};
use pes_version::PesVersion;
use pipeline::{MemoryBudget, Permit};

use super::{Entry, Finding, TaskFailure, TaskFiles, take, texture};
use crate::messages::Code;
use crate::paths;
use crate::plan::roles::texture_format;

/// The largest logo the main file feeds, and so the side under which it is upscaled.
const MAIN_LARGEST: u32 = 512;
/// The one logo the small file feeds, the 128-pixel one.
const SMALL_SIDE: u32 = 128;

/// One logo file, decoded: its export path, the mode that makes it square, and its top level.
struct LogoSource<'a> {
    path: &'a str,
    fit: LogoFit,
    width: u32,
    height: u32,
    /// Straight-alpha RGBA8, `width` x `height`, row-major.
    pixels: Vec<u8>,
    /// The decode's charge to the run's budget, released with the pixels.
    _charge: Permit,
}

/// The team `team_id`'s three logo PNGs for `version`, all or none, from `logo`'s files, whose
/// bytes are in `files`: the 512- and 256-pixel ones from the main file, the 128-pixel one from
/// the small file when there is one and from the main file otherwise, each resampled from its
/// source directly. Noted in `findings`, the main file first: `logo_fit_applied` for a
/// non-square source, naming its mode, and `logo_upscaled` for a source whose side the mode
/// maps onto the target is under the largest target it feeds. A file that does not decode, or
/// a size the resampler or the encoder refuses, fails the task as a texture's conversion does.
/// Each file's decode is charged to `budget` while it lives; the three squares, under 2 MiB
/// together at 512, 256 and 128 pixels, are not.
pub(super) fn logo(
    logo: &LogoFiles,
    team_id: u16,
    version: PesVersion,
    budget: &Arc<MemoryBudget>,
    files: &mut TaskFiles,
    findings: &mut Vec<Finding>,
) -> Result<Vec<Entry>, TaskFailure> {
    let main = decoded(&logo.main, budget, files)?;
    let small = logo
        .small
        .as_ref()
        .map(|file| decoded(file, budget, files))
        .transpose()?;
    report(&main, MAIN_LARGEST, findings);
    if let Some(small) = &small {
        report(small, SMALL_SIDE, findings);
    }
    // In `paths::logo`'s order.
    let targets = [
        (&main, MAIN_LARGEST),
        (&main, 256),
        (small.as_ref().unwrap_or(&main), SMALL_SIDE),
    ];
    let mut entries = Vec::with_capacity(targets.len());
    for (path, (source, side)) in paths::logo(version, team_id).into_iter().zip(targets) {
        let png = squared(source, side)
            .and_then(|pixels| encode_png(&pixels, side, side))
            .map_err(|error| texture::conversion_failure(source.path, error))?;
        entries.push((path, png));
    }
    Ok(entries)
}

/// The logo `file`, its bytes taken out of `files`, decoded in the format its extension names:
/// a DDS or FTEX source gives its top level. The decode is charged to `budget`, every level
/// the source carries, until the logo source is dropped.
fn decoded<'a>(
    file: &'a LogoFile,
    budget: &Arc<MemoryBudget>,
    files: &mut TaskFiles,
) -> Result<LogoSource<'a>, TaskFailure> {
    let path = file.file.path.as_str();
    let format = texture_format(file.file.path.name())
        .expect("validation keeps a logo only in a format `dds_convert` accepts");
    let bytes = take(files, &file.file);
    let failure = |error| texture::conversion_failure(path, error);
    let charge = texture::decode_charge(budget, &bytes, format).map_err(failure)?;
    let decoded = decode(&bytes, format).map_err(failure)?;
    let pixels = decoded
        .mips
        .into_iter()
        .next()
        .expect("a decoded texture has its top level");
    Ok(LogoSource {
        path,
        fit: file.fit.unwrap_or(LogoFit::Fit),
        width: decoded.width,
        height: decoded.height,
        pixels,
        _charge: charge,
    })
}

/// Notes in `findings` what making `source` into logos up to `largest` pixels does to it:
/// `logo_fit_applied` when it is not square, and `logo_upscaled` when the side its mode maps
/// onto the target is under `largest`.
fn report(source: &LogoSource, largest: u32, findings: &mut Vec<Finding>) {
    let long = source.width.max(source.height);
    let short = source.width.min(source.height);
    if long != short {
        findings.push((
            Code::LogoFitApplied,
            vec![
                ("file", source.path.to_owned()),
                ("mode", mode_name(source.fit).to_owned()),
            ],
        ));
    }
    // `fit` keeps the whole image inside the square, so its long side is the one stretched to
    // reach the target; `crop` and `stretch` fill the square, so their short side is.
    let mapped = match source.fit {
        LogoFit::Fit => long,
        LogoFit::Crop | LogoFit::Stretch => short,
    };
    if mapped < largest {
        findings.push((
            Code::LogoUpscaled,
            vec![
                ("file", source.path.to_owned()),
                ("size", format!("{}x{}", source.width, source.height)),
                ("target", largest.to_string()),
            ],
        ));
    }
}

/// The name of a logo mode, as the file's tag spells it and `logo_fit_applied` reports it.
fn mode_name(fit: LogoFit) -> &'static str {
    match fit {
        LogoFit::Fit => "fit",
        LogoFit::Crop => "crop",
        LogoFit::Stretch => "stretch",
    }
}

/// `source` made into `side` x `side` pixels by its mode. A square source is resampled as it
/// is whatever its mode, and one already at that size comes back unfiltered (`resize` copies a
/// size it does not change).
fn squared(source: &LogoSource, side: u32) -> Result<Vec<u8>, ConvertError> {
    let (width, height, pixels) = (source.width, source.height, &source.pixels);
    if width == height {
        return resize(pixels, width, height, side, side);
    }
    match source.fit {
        LogoFit::Stretch => resize(pixels, width, height, side, side),
        LogoFit::Crop => {
            let (square, short) = centre_square(pixels, width, height);
            resize(&square, short, short, side, side)
        }
        LogoFit::Fit => letterboxed(pixels, width, height, side),
    }
}

/// The centre square of a non-square `width` x `height` image, and its side: the long side cut
/// to the short one, the cut starting at half the difference, rounded down.
fn centre_square(pixels: &[u8], width: u32, height: u32) -> (Vec<u8>, u32) {
    let short = width.min(height);
    let row_len = width as usize * 4;
    let square_row_len = short as usize * 4;
    if width > height {
        let start = (width - height) as usize / 2 * 4;
        let square = pixels
            .chunks_exact(row_len)
            .flat_map(|row| &row[start..start + square_row_len])
            .copied()
            .collect();
        (square, short)
    } else {
        let start = (height - width) as usize / 2 * row_len;
        (
            pixels[start..start + short as usize * row_len].to_vec(),
            short,
        )
    }
}

/// A non-square `width` x `height` image resampled with its proportions kept, the long side to
/// `side` and the short one to `side * short / long` (rounded half up, at least 1), and centred
/// (the offset rounded down) on a `side` x `side` canvas of transparent black. Resampled before
/// it is padded, not after: the filter run over the padded image would blend the logo's edge
/// with the border and leave a dark fringe.
fn letterboxed(pixels: &[u8], width: u32, height: u32, side: u32) -> Result<Vec<u8>, ConvertError> {
    let long = u64::from(width.max(height));
    let short = u64::from(width.min(height));
    let scaled = ((u64::from(side) * short + long / 2) / long).max(1);
    let scaled = u32::try_from(scaled)
        .expect("the short side, scaled as the long one is to `side`, is at most `side`");
    let offset = ((side - scaled) / 2) as usize;
    let canvas_row_len = side as usize * 4;
    let mut canvas = vec![0; side as usize * canvas_row_len];
    if width > height {
        // The rows are whole canvas rows, so the image goes in as one block.
        let resized = resize(pixels, width, height, side, scaled)?;
        let start = offset * canvas_row_len;
        canvas[start..start + resized.len()].copy_from_slice(&resized);
    } else {
        let resized = resize(pixels, width, height, scaled, side)?;
        let row_len = scaled as usize * 4;
        for (row, canvas_row) in resized
            .chunks_exact(row_len)
            .zip(canvas.chunks_exact_mut(canvas_row_len))
        {
            canvas_row[offset * 4..offset * 4 + row_len].copy_from_slice(row);
        }
    }
    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use aesthetics_export::FileDescriptor;
    use dds_convert::{BlockCodec, Blocks, Decoded, SourceFormat, encode_dds};
    use vtree::ScopePath;

    use super::*;

    const RED: [u8; 4] = [255, 0, 0, 255];
    const GREEN: [u8; 4] = [0, 255, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

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

    /// `image` as a PNG file.
    fn png(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        encode_png(&image(width, height, color), width, height).unwrap()
    }

    /// A flat `color` PNG of `width` x `height`.
    fn flat(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
        png(width, height, |_, _| color)
    }

    /// The root file `name` holding `bytes`, tagged `fit`.
    fn logo_file(name: &str, fit: Option<LogoFit>, bytes: &[u8]) -> LogoFile {
        let path = ScopePath::new(name).unwrap();
        LogoFile {
            file: FileDescriptor {
                size: bytes.len() as u64,
                kind: aesthetics_export::classify(path.name()),
                source: path.clone(),
                path,
            },
            fit,
        }
    }

    /// One logo source for `run`: its file name, its tag and its bytes.
    type Source<'a> = (&'a str, Option<LogoFit>, Vec<u8>);

    /// One PNG `run` gives: its path, its side and its decoded pixels.
    type Output = (String, u32, Vec<u8>);

    /// What `logo` makes of the main file `main` and the small file `small` for team 714 on
    /// PES 21: each PNG, and the findings.
    fn run(main: Source, small: Option<Source>) -> (Vec<Output>, Vec<Finding>) {
        let mut files = TaskFiles::new();
        let mut file = |(name, fit, bytes): Source| {
            let file = logo_file(name, fit, &bytes);
            files.insert(file.file.path.clone(), bytes);
            file
        };
        let logo_files = LogoFiles {
            main: file(main),
            small: small.map(file),
        };
        let mut findings = Vec::new();
        let entries = match logo(
            &logo_files,
            714,
            PesVersion::Pes21,
            &pipeline::MemoryBudget::new(usize::MAX),
            &mut files,
            &mut findings,
        ) {
            Ok(entries) => entries,
            Err(failure) => panic!("{:?} {:?}", failure.code, failure.context),
        };
        let outputs = entries
            .into_iter()
            .map(|(path, bytes)| {
                let mut decoded = decode(&bytes, SourceFormat::Png).unwrap();
                assert_eq!(decoded.width, decoded.height, "{path}");
                (path, decoded.width, decoded.mips.swap_remove(0))
            })
            .collect();
        (outputs, findings)
    }

    /// The texel at (x, y) of `pixels`, `side` pixels wide.
    fn texel(pixels: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * side + x) * 4) as usize;
        pixels[at..at + 4].try_into().unwrap()
    }

    /// Asserts every texel of the `side`-pixel `pixels` is `expected(x, y)`, naming the first
    /// that is not.
    fn assert_texels(pixels: &[u8], side: u32, expected: impl Fn(u32, u32) -> [u8; 4]) {
        assert_eq!(pixels.len(), (side * side * 4) as usize);
        for y in 0..side {
            for x in 0..side {
                assert_eq!(
                    texel(pixels, side, x, y),
                    expected(x, y),
                    "({x}, {y}) of {side}"
                );
            }
        }
    }

    fn fit_applied(file: &str, mode: &str) -> Finding {
        (
            Code::LogoFitApplied,
            vec![("file", file.to_owned()), ("mode", mode.to_owned())],
        )
    }

    fn upscaled(file: &str, size: &str, target: &str) -> Finding {
        (
            Code::LogoUpscaled,
            vec![
                ("file", file.to_owned()),
                ("size", size.to_owned()),
                ("target", target.to_owned()),
            ],
        )
    }

    /// The paths of PES 21's three logos of team 714, largest first.
    const PATHS: [&str; 3] = [
        "common/render/symbol/flag/e_000714_r_ll.png",
        "common/render/symbol/flag/e_000714_r_l.png",
        "common/render/symbol/flag/e_000714_r.png",
    ];

    #[test]
    fn an_untagged_wide_logo_is_fitted_between_transparent_rows() {
        let (outputs, findings) = run(("logo.png", None, flat(1000, 600, RED)), None);

        assert_eq!(findings, [fit_applied("logo.png", "fit")]);
        // 512 x 307 at row 102, 256 x 154 at 51, 128 x 77 at 25.
        let expected = [(512, 102..409), (256, 51..205), (128, 25..102)];
        assert_eq!(outputs.len(), 3);
        for ((path, side, pixels), (expected_side, rows)) in outputs.iter().zip(expected) {
            assert_eq!(*side, expected_side, "{path}");
            assert_texels(
                pixels,
                *side,
                |_, y| {
                    if rows.contains(&y) { RED } else { CLEAR }
                },
            );
        }
        let paths: Vec<&str> = outputs.iter().map(|(path, ..)| path.as_str()).collect();
        assert_eq!(paths, PATHS);
    }

    #[test]
    fn a_fitted_logo_keeps_at_least_one_row() {
        // 2000 x 1: the short side scales to 512 / 2000, rounded to 0 at every size.
        let (outputs, _) = run(("logo.png", None, flat(2000, 1, RED)), None);

        for (_, side, pixels) in &outputs {
            let row = (side - 1) / 2;
            assert_texels(pixels, *side, |_, y| if y == row { RED } else { CLEAR });
        }
    }

    #[test]
    fn a_tall_logo_tagged_fit_is_fitted_between_transparent_columns() {
        let (outputs, findings) = run(
            ("logo_fit.png", Some(LogoFit::Fit), flat(600, 1000, RED)),
            None,
        );

        assert_eq!(findings, [fit_applied("logo_fit.png", "fit")]);
        let expected = [(512, 102..409), (256, 51..205), (128, 25..102)];
        for ((_, side, pixels), (expected_side, columns)) in outputs.iter().zip(expected) {
            assert_eq!(*side, expected_side);
            assert_texels(pixels, *side, |x, _| {
                if columns.contains(&x) { RED } else { CLEAR }
            });
        }
    }

    /// Blue up to 200, red from 200 to 800, green past it.
    fn three_bands(at: u32) -> [u8; 4] {
        match at {
            0..200 => BLUE,
            200..800 => RED,
            _ => GREEN,
        }
    }

    #[test]
    fn a_logo_tagged_crop_keeps_its_centre_square() {
        let wide = png(1000, 600, |x, _| three_bands(x));
        let tall = png(600, 1000, |_, y| three_bands(y));

        for source in [wide, tall] {
            let (outputs, findings) = run(("logo_crop.png", Some(LogoFit::Crop), source), None);

            assert_eq!(findings, [fit_applied("logo_crop.png", "crop")]);
            let sides: Vec<u32> = outputs.iter().map(|(_, side, _)| *side).collect();
            assert_eq!(sides, [512, 256, 128]);
            for (_, side, pixels) in &outputs {
                assert_texels(pixels, *side, |_, _| RED);
            }
        }
    }

    #[test]
    fn a_wide_logo_tagged_stretch_fills_the_square() {
        let halves = png(1000, 600, |x, _| if x < 500 { RED } else { BLUE });

        let (outputs, findings) = run(("logo_stretch.png", Some(LogoFit::Stretch), halves), None);

        assert_eq!(findings, [fit_applied("logo_stretch.png", "stretch")]);
        for (_, side, pixels) in &outputs {
            let row_len = (*side * 4) as usize;
            let first_row = &pixels[..row_len];
            for (y, row) in pixels.chunks_exact(row_len).enumerate() {
                assert_eq!(row, first_row, "row {y} of {side}");
            }
            for x in 0..*side {
                assert_eq!(texel(pixels, *side, x, 0)[3], 255, "({x}, 0) of {side}");
            }
            assert_eq!(texel(pixels, *side, 0, 0), RED, "{side}");
            assert_eq!(texel(pixels, *side, side - 1, 0), BLUE, "{side}");
        }
    }

    #[test]
    fn a_square_logo_is_not_fitted_whatever_its_tag_and_a_small_one_is_upscaled() {
        let (outputs, findings) = run(
            ("logo_crop.png", Some(LogoFit::Crop), flat(300, 300, GREEN)),
            None,
        );

        assert_eq!(findings, [upscaled("logo_crop.png", "300x300", "512")]);
        for (_, side, pixels) in &outputs {
            assert_texels(pixels, *side, |_, _| GREEN);
        }
    }

    #[test]
    fn a_logo_is_upscaled_when_the_side_its_mode_maps_is_under_its_largest_target() {
        let findings = |main: Source, small: Option<Source>| run(main, small).1;

        // 1000 x 500: `fit` maps the 1000 side, `crop` and `stretch` the 500 one.
        assert_eq!(
            findings(("logo.png", None, flat(1000, 500, RED)), None),
            [fit_applied("logo.png", "fit")]
        );
        for (name, fit, mode) in [
            ("logo_crop.png", LogoFit::Crop, "crop"),
            ("logo_stretch.png", LogoFit::Stretch, "stretch"),
        ] {
            assert_eq!(
                findings((name, Some(fit), flat(1000, 500, RED)), None),
                [fit_applied(name, mode), upscaled(name, "1000x500", "512")]
            );
        }
        assert_eq!(
            findings(("logo.png", None, flat(500, 100, RED)), None),
            [
                fit_applied("logo.png", "fit"),
                upscaled("logo.png", "500x100", "512")
            ]
        );
        // At their largest target exactly, neither is upscaled; the small one is measured
        // against 128.
        assert_eq!(
            findings(
                ("logo.png", None, flat(512, 512, RED)),
                Some(("logo_small.png", None, flat(128, 128, BLUE)))
            ),
            []
        );
        assert_eq!(
            findings(
                ("logo.png", None, flat(512, 512, RED)),
                Some(("logo_small.png", None, flat(100, 100, BLUE)))
            ),
            [upscaled("logo_small.png", "100x100", "128")]
        );
    }

    #[test]
    fn the_small_logo_feeds_the_128_pixel_one_alone() {
        let (outputs, findings) = run(
            ("logo.png", None, flat(600, 600, RED)),
            Some(("logo_small.png", None, flat(200, 200, BLUE))),
        );

        assert_eq!(findings, []);
        let colors: Vec<(u32, [u8; 4])> = outputs
            .iter()
            .map(|(_, side, pixels)| {
                let color = texel(pixels, *side, 0, 0);
                assert_texels(pixels, *side, |_, _| color);
                (*side, color)
            })
            .collect();
        assert_eq!(colors, [(512, RED), (256, RED), (128, BLUE)]);
    }

    #[test]
    fn a_source_already_at_512_pixels_comes_back_texel_for_texel() {
        // Every texel different, alpha included: (x, y) is readable from the first three bytes.
        let color = |x: u32, y: u32| {
            [
                x as u8,
                y as u8,
                ((x >> 8) | ((y >> 8) << 1)) as u8,
                (x * 7 + y * 13) as u8,
            ]
        };

        let (outputs, _) = run(("logo.png", None, png(512, 512, color)), None);

        assert_eq!(outputs[0].1, 512);
        assert_eq!(outputs[0].2, image(512, 512, color));
    }

    /// A 64 x 64 BC1 DDS with four levels: the top one flat in the 5:6:5 color `top`, the three
    /// below flat in `lower`.
    fn four_level_dds(top: u16, lower: u16) -> Vec<u8> {
        // Both endpoints the one color and every index 0: each pixel is the first endpoint.
        let block = |color: u16| {
            let [low, high] = color.to_le_bytes();
            [low, high, low, high, 0, 0, 0, 0]
        };
        let sides = [64usize, 32, 16, 8];
        let decoded = Decoded {
            width: 64,
            height: 64,
            mips: sides.iter().map(|side| vec![0; side * side * 4]).collect(),
            blocks: Some(Blocks {
                codec: BlockCodec::Bc1,
                mips: sides
                    .iter()
                    .enumerate()
                    .map(|(level, side)| {
                        let color = if level == 0 { top } else { lower };
                        block(color).repeat((side / 4) * (side / 4))
                    })
                    .collect(),
            }),
            authored_mips: true,
            uncompressed: false,
        };
        encode_dds(&decoded, BlockCodec::Bc1).unwrap()
    }

    #[test]
    fn a_dds_logo_with_a_mip_chain_gives_its_top_level() {
        let dds = four_level_dds(0xc28a, 0x001f);
        let levels = decode(&dds, SourceFormat::Dds).unwrap().mips;
        assert_eq!(levels.len(), 4);
        let top = texel(&levels[0], 64, 0, 0);
        assert_ne!(top, texel(&levels[1], 32, 0, 0), "the levels differ");

        let (outputs, findings) = run(("logo.dds", None, dds), None);

        assert_eq!(findings, [upscaled("logo.dds", "64x64", "512")]);
        for (_, side, pixels) in &outputs {
            assert_texels(pixels, *side, |_, _| top);
        }
    }
}
