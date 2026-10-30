//! An event's logo as the Save editor and the Ruleset editor draw it: the
//! picture a ruleset carries, decoded once into a texture and drawn 28
//! points high, the sidebar clover's height, as wide as its shape asks up
//! to three times that.

/// The height of the box a logo is drawn in, in points: the sidebar
/// clover's. A picture wider than 3:1 is drawn less high, in its middle.
pub const HEIGHT: f32 = 28.0;

/// The widest a logo is drawn, in points: three times its height, so a
/// wide logo still reads.
const MAX_WIDTH: f32 = 84.0;

/// An event logo decoded and uploaded: built once per logo (a ruleset
/// change, a new pick), never per frame.
pub struct EventLogo {
    /// The uploaded picture.
    texture: egui::TextureHandle,
    /// The picture's size in pixels.
    pixels: egui::Vec2,
}

impl EventLogo {
    /// `png` decoded and uploaded as the texture `name`; `None` when it
    /// does not decode, and then nothing is drawn: a logo whose bytes
    /// passed the ruleset's load check can still hold broken pixels.
    pub fn load(ctx: &egui::Context, name: &str, png: &[u8]) -> Option<EventLogo> {
        let Ok(decoded) = dds_convert::decode(png, dds_convert::SourceFormat::Png) else {
            return None;
        };
        let pixels = decoded.mips.first()?;
        let size = [decoded.width as usize, decoded.height as usize];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, pixels);
        Some(EventLogo {
            texture: ctx.load_texture(name, image, egui::TextureOptions::LINEAR),
            pixels: egui::vec2(decoded.width as f32, decoded.height as f32),
        })
    }

    /// The size the picture is drawn at: fitted into a box 28 high and up
    /// to 84 wide, its proportions kept (a 2:1 picture is 56 by 28, a 4:1
    /// one 84 by 21).
    pub fn size(&self) -> egui::Vec2 {
        let scale = (HEIGHT / self.pixels.y).min(MAX_WIDTH / self.pixels.x);
        self.pixels * scale
    }

    /// The picture as an image widget at `size`, for the host to add.
    pub fn image(&self) -> egui::Image<'static> {
        egui::Image::from_texture(egui::load::SizedTexture::new(
            self.texture.id(),
            self.size(),
        ))
    }
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

    #[test]
    fn a_logo_that_does_not_decode_is_none() {
        // The PNG signature, then no valid chunk.
        let broken = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
        assert!(EventLogo::load(&egui::Context::default(), "logo", &broken).is_none());
    }
}
