//! An event's logo as the Save editor and the Ruleset editor draw it: the
//! picture a ruleset carries, decoded once into a texture and drawn 28
//! points high, the sidebar clover's height, as wide as its shape asks up
//! to three times that.
//!
//! The texture holds the picture resampled with Lanczos3 to the pixels its
//! box takes at the window's scale, and again when the scale changes, so
//! it is drawn at one texel per pixel. Left to the GPU, the shrinking
//! happens as it draws: linear sampling with no mipmaps reads only the
//! texels nearest each pixel, so past a halving it skips source pixels and
//! the logo reads pixelated (a 78 × 84 logo drawn 28 points high at 100%).

/// The height of the box a logo is drawn in, in points: the sidebar
/// clover's. A picture wider than 3:1 is drawn less high, in its middle.
pub const HEIGHT: f32 = 28.0;

/// The widest a logo is drawn, in points: three times its height, so a
/// wide logo still reads.
const MAX_WIDTH: f32 = 84.0;

/// An event logo decoded and uploaded: decoded once per logo (a ruleset
/// change, a new pick), never per frame. The texture is the picture
/// resampled to the pixels it is drawn at, not the picture itself, so
/// the GPU never shrinks it (the module's comment says why); a change of
/// the window's scale resamples it once more.
pub struct EventLogo {
    /// The uploaded picture, resampled for `pixels_per_point`.
    texture: egui::TextureHandle,
    /// The decoded picture, straight-alpha RGBA8, row-major: what every
    /// resampling starts from.
    source: Vec<u8>,
    /// `source`'s width in pixels.
    source_width: u32,
    /// `source`'s height in pixels.
    source_height: u32,
    /// The window's scale (physical pixels per point) `texture` was
    /// resampled for.
    pixels_per_point: f32,
}

impl EventLogo {
    /// `png` decoded and uploaded as the texture `name`, resampled for the
    /// scale of `ctx`'s current frame; `None` when it does not decode, and
    /// then nothing is drawn: a logo whose bytes passed the ruleset's load
    /// check can still hold broken pixels.
    pub fn load(ctx: &egui::Context, name: &str, png: &[u8]) -> Option<EventLogo> {
        let decoded = match dds_convert::decode(png, dds_convert::SourceFormat::Png) {
            Ok(decoded) => decoded,
            Err(error) => {
                log::debug!("the event logo {name} is not drawn: its PNG does not decode: {error}");
                return None;
            }
        };
        let source = decoded.mips.into_iter().next()?;
        let pixels_per_point = ctx.pixels_per_point();
        let image = resampled(&source, decoded.width, decoded.height, pixels_per_point);
        Some(EventLogo {
            texture: ctx.load_texture(name, image, egui::TextureOptions::LINEAR),
            source,
            source_width: decoded.width,
            source_height: decoded.height,
            pixels_per_point,
        })
    }

    /// The size the picture is drawn at: fitted into a box 28 high and up
    /// to 84 wide, its proportions kept (a 2:1 picture is 56 by 28, a 4:1
    /// one 84 by 21).
    pub fn size(&self) -> egui::Vec2 {
        drawn_size(self.source_width, self.source_height)
    }

    /// The picture as an image widget at `size`, for the host to add on
    /// `ctx`'s current frame. When the window's scale differs from the one
    /// the texture was resampled for (the window moved to a screen of
    /// another scaling), the picture is resampled for the new one first,
    /// into the same texture.
    pub fn image(&mut self, ctx: &egui::Context) -> egui::Image<'static> {
        let pixels_per_point = ctx.pixels_per_point();
        if pixels_per_point != self.pixels_per_point {
            let image = resampled(
                &self.source,
                self.source_width,
                self.source_height,
                pixels_per_point,
            );
            self.texture.set(image, egui::TextureOptions::LINEAR);
            self.pixels_per_point = pixels_per_point;
        }
        egui::Image::from_texture(egui::load::SizedTexture::new(
            self.texture.id(),
            self.size(),
        ))
    }
}

/// The size, in points, a `width` × `height` picture is drawn at (see
/// `EventLogo::size`).
fn drawn_size(width: u32, height: u32) -> egui::Vec2 {
    let pixels = egui::vec2(width as f32, height as f32);
    let scale = (HEIGHT / pixels.y).min(MAX_WIDTH / pixels.x);
    pixels * scale
}

/// `source` (`width` × `height`, straight-alpha RGBA8) resampled with
/// Lanczos3 on premultiplied colour to the pixels its drawn size takes at
/// `pixels_per_point`, each side rounded to the nearest pixel and at least
/// 1: resampled straight, the colour a transparent texel stores (black, in
/// most logos) would bleed into the edge as a dark fringe.
fn resampled(source: &[u8], width: u32, height: u32, pixels_per_point: f32) -> egui::ColorImage {
    let drawn = drawn_size(width, height) * pixels_per_point;
    let side = |points: f32| (points.round() as u32).max(1);
    let (new_width, new_height) = (side(drawn.x), side(drawn.y));
    let pixels = dds_convert::resize_straight_alpha(source, width, height, new_width, new_height)
        .expect(
            "a decoded PNG is a whole picture of at least 1 × 1 and both new sides are at least \
             1: the resampler refuses only a zero side or a buffer of the wrong length",
        );
    egui::ColorImage::from_rgba_unmultiplied([new_width as usize, new_height as usize], &pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain picture `width` by `height` pixels, as PNG bytes.
    fn png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![200; (width * height * 4) as usize];
        dds_convert::encode_png(&pixels, width, height).expect("a plain picture encodes")
    }

    /// The drawn size of a `width` by `height` picture.
    fn size_of(width: u32, height: u32) -> egui::Vec2 {
        EventLogo::load(&egui::Context::default(), "logo", &png(width, height))
            .expect("the picture decodes")
            .size()
    }

    #[test]
    fn a_square_logo_is_drawn_28_by_28() {
        assert_eq!(size_of(84, 84), egui::vec2(28.0, 28.0));
    }

    #[test]
    fn a_2_to_1_logo_is_drawn_56_by_28() {
        assert_eq!(size_of(160, 80), egui::vec2(56.0, 28.0));
    }

    #[test]
    fn a_4_to_1_logo_is_capped_at_84_wide() {
        assert_eq!(size_of(160, 40), egui::vec2(84.0, 21.0));
    }

    #[test]
    fn a_1_to_2_logo_is_drawn_14_by_28() {
        assert_eq!(size_of(40, 80), egui::vec2(14.0, 28.0));
    }

    /// Runs one frame of `ctx` on a screen whose scale is `pixels_per_point`
    /// (the viewport's native scale, as a monitor's scaling reaches egui).
    fn frame_at_scale(
        ctx: &egui::Context,
        pixels_per_point: f32,
        contents: impl FnMut(&mut egui::Ui),
    ) {
        let mut input = egui::RawInput::default();
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(pixels_per_point);
        let output = ctx.run_ui(input, contents);
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
    }

    /// DBC7's logo, loaded on a frame at `pixels_per_point`.
    fn dbc7_logo_at(ctx: &egui::Context, pixels_per_point: f32) -> EventLogo {
        let png = aatf::builtin("dbc7")
            .expect("dbc7 is built in")
            .ruleset()
            .logo_png()
            .expect("dbc7 carries a logo");
        let decoded =
            dds_convert::decode(&png, dds_convert::SourceFormat::Png).expect("dbc7's logo decodes");
        assert_eq!(
            (decoded.width, decoded.height),
            (78, 84),
            "premise: dbc7's logo is 78 by 84 pixels"
        );
        let mut logo = None;
        frame_at_scale(ctx, pixels_per_point, |ui| {
            logo = EventLogo::load(ui.ctx(), "dbc7", &png);
        });
        logo.expect("dbc7's logo decodes")
    }

    #[test]
    fn dbc7s_logo_is_resampled_to_the_pixels_it_is_drawn_at() {
        let at_100 = dbc7_logo_at(&egui::Context::default(), 1.0);
        assert_eq!(at_100.texture.size(), [26, 28], "at 100% scaling");
        let at_200 = dbc7_logo_at(&egui::Context::default(), 2.0);
        assert_eq!(at_200.texture.size(), [52, 56], "at 200% scaling");
    }

    #[test]
    fn a_scale_change_resamples_the_logo_into_the_same_texture() {
        let ctx = egui::Context::default();
        let mut logo = dbc7_logo_at(&ctx, 1.0);
        let id = logo.texture.id();
        assert_eq!(logo.texture.size(), [26, 28], "premise: built at 100%");
        frame_at_scale(&ctx, 2.0, |ui| {
            ui.add(logo.image(ui.ctx()));
        });
        assert_eq!(logo.texture.size(), [52, 56], "redrawn at 200% scaling");
        assert_eq!(logo.texture.id(), id, "the texture keeps its id");
    }

    #[test]
    fn an_edge_against_transparent_black_is_not_darkened() {
        let (width, height) = (56, 84);
        let mut straight = Vec::new();
        for _ in 0..height {
            for x in 0..width {
                let texel = if x < 28 { [255; 4] } else { [0; 4] };
                straight.extend_from_slice(&texel);
            }
        }
        let image = resampled(&straight, width, height, 1.0);
        assert_eq!(image.size, [19, 28], "premise: drawn 18.67 × 28, rounded");
        for (index, texel) in image.pixels.iter().enumerate() {
            let [r, g, b, a] = texel.to_srgba_unmultiplied();
            if a >= 16 {
                assert!(
                    r >= 240 && g >= 240 && b >= 240,
                    "texel {index} (x {}) reads {:?}",
                    index % 19,
                    [r, g, b, a]
                );
            }
        }
    }

    #[test]
    fn a_logo_that_does_not_decode_is_none() {
        // The PNG signature, then no valid chunk.
        let broken = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
        assert!(EventLogo::load(&egui::Context::default(), "logo", &broken).is_none());
    }
}
