//! The pictures the grid's detail window shows: each export file a top box names, read and
//! decoded on a thread of its own, scaled down and uploaded as a texture once. A frame only
//! looks a picture up, or sends it to the thread the first time it is asked for.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Context as _;
use crossbeam_channel::{Receiver, Sender};
use dds_convert::SourceFormat;
use export_grid::PictureFile;

/// The longest side a picture reaches the window with. The top box draws none larger than
/// 180 points, and a kit's texture, which can be 2048 texels square, must not reach the GPU
/// whole.
const LONGEST_SIDE: u32 = 256;

/// Where one picture stands.
enum Picture {
    /// Sent to the worker, not answered yet.
    Loading,
    /// Read, decoded and uploaded.
    Loaded(egui::TextureHandle),
    /// It could not be read or decoded: asked once, never again.
    Unreadable,
}

/// The detail window's pictures: each file's texture once the worker has read it, kept until
/// `clear`.
#[derive(Default)]
pub(crate) struct Pictures {
    pictures: HashMap<PictureFile, Picture>,
    /// The reading thread, started at the first ask and stopped by `clear`.
    worker: Option<Worker>,
}

/// The reading thread, as its channels: files in, decoded images out (`None`: the file could
/// not be read or decoded).
struct Worker {
    asks: Sender<PictureFile>,
    answers: Receiver<(PictureFile, Option<egui::ColorImage>)>,
}

impl Worker {
    /// A thread that reads each file sent to it, until the sender drops, and asks `ctx` for a
    /// repaint after each answer.
    fn spawn(ctx: &egui::Context) -> Worker {
        let (asks, asked) = crossbeam_channel::unbounded::<PictureFile>();
        let (answer, answers) = crossbeam_channel::unbounded();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            for file in asked {
                let image = match read(&file) {
                    Ok(image) => Some(image),
                    Err(error) => {
                        log::debug!("{}: {error:#}", file.path);
                        None
                    }
                };
                // The send fails once `clear` has dropped the receiver: the answer is stale.
                if answer.send((file, image)).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
        Worker { asks, answers }
    }
}

impl Pictures {
    /// `file`'s texture, or `None` while it loads or when it cannot be read. The first ask
    /// sends it to the worker.
    pub(crate) fn texture(
        &mut self,
        ctx: &egui::Context,
        file: &PictureFile,
    ) -> Option<egui::TextureHandle> {
        match self.pictures.get(file) {
            Some(Picture::Loaded(texture)) => return Some(texture.clone()),
            Some(Picture::Loading | Picture::Unreadable) => return None,
            None => {}
        }
        let worker = self.worker.get_or_insert_with(|| Worker::spawn(ctx));
        // The send fails only when the worker has stopped; the picture then stays loading.
        if worker.asks.send(file.clone()).is_err() {
            log::debug!("the pictures' worker has stopped");
        }
        self.pictures.insert(file.clone(), Picture::Loading);
        None
    }

    /// Uploads the pictures the worker has answered since the last call.
    pub(crate) fn receive(&mut self, ctx: &egui::Context) {
        let Some(worker) = &self.worker else {
            return;
        };
        for (file, image) in worker.answers.try_iter() {
            let picture = match image {
                Some(image) => Picture::Loaded(ctx.load_texture(
                    file.path.clone(),
                    image,
                    egui::TextureOptions::LINEAR,
                )),
                None => Picture::Unreadable,
            };
            self.pictures.insert(file, picture);
        }
    }

    /// Forgets every picture, so each is read again when next asked for: the files may have
    /// changed since. The worker stops, and the answers it still had on the way are dropped.
    pub(crate) fn clear(&mut self) {
        self.pictures.clear();
        self.worker = None;
    }

    /// Whether `file` was asked for and its answer has not been received yet.
    #[cfg(test)]
    pub(crate) fn is_loading(&self, file: &PictureFile) -> bool {
        matches!(self.pictures.get(file), Some(Picture::Loading))
    }
}

/// `file` read and decoded, scaled down so its longer side is at most `LONGEST_SIDE`.
fn read(file: &PictureFile) -> anyhow::Result<egui::ColorImage> {
    let format = Path::new(&file.path)
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(SourceFormat::from_extension)
        .context("not an image the Compiler reads")?;
    let bytes = fs::read(file.export.join(&file.path))?;
    let decoded = dds_convert::decode(&bytes, format)?;
    let (pixels, width, height) = small_enough(&decoded)?;
    let size = [width as usize, height as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, &pixels))
}

/// `decoded`'s pixels with a longer side of at most `LONGEST_SIDE`, and their width and
/// height: the smallest mip the texture carries that is still at least that long, scaled
/// down the rest of the way.
fn small_enough(decoded: &dds_convert::Decoded) -> anyhow::Result<(Vec<u8>, u32, u32)> {
    let side = |length: u32, level: usize| (length >> level).max(1);
    let mut level = 0;
    while level + 1 < decoded.mips.len()
        && side(decoded.width, level + 1).max(side(decoded.height, level + 1)) >= LONGEST_SIDE
    {
        level += 1;
    }
    let (width, height) = (side(decoded.width, level), side(decoded.height, level));
    let pixels = decoded
        .mips
        .get(level)
        .context("a texture with no pixels")?;
    // Checked here: an image of the wrong length panics on its way to a texture.
    anyhow::ensure!(
        pixels.len() == width as usize * height as usize * 4,
        "mip {level} does not fill {width} × {height}"
    );
    let longer = width.max(height);
    if longer <= LONGEST_SIDE {
        return Ok((pixels.clone(), width, height));
    }
    let scaled = |length: u32| ((length * LONGEST_SIDE + longer / 2) / longer).max(1);
    let (new_width, new_height) = (scaled(width), scaled(height));
    // Portraits and logos are cut out on transparency: a straight-alpha resize keeps a
    // transparent texel's colour from bleeding into its neighbours.
    let resized = dds_convert::resize_straight_alpha(pixels, width, height, new_width, new_height)?;
    Ok((resized, new_width, new_height))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use dds_convert::BlockCodec;

    use super::*;
    use crate::testing::scratch;

    /// A PNG `width` × `height`, opaque white.
    fn png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![255; width as usize * height as usize * 4];
        dds_convert::encode_png(&pixels, width, height).unwrap()
    }

    /// A DXT1 DDS `width` × `height` with its whole mip chain.
    fn dds(width: u32, height: u32) -> Vec<u8> {
        let decoded = dds_convert::decode(&png(width, height), SourceFormat::Png).unwrap();
        dds_convert::encode_dds(&decoded, BlockCodec::Bc1).unwrap()
    }

    /// The picture `name` of the export folder `export`.
    fn picture(export: &Path, name: &str) -> PictureFile {
        PictureFile {
            export: export.to_owned(),
            path: name.to_owned(),
        }
    }

    /// Receives the worker's answers until none of `files` is loading, a real nap between
    /// tries: a hang guard of 120 s, not a speed bound.
    fn answered(pictures: &mut Pictures, ctx: &egui::Context, files: &[&PictureFile]) {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            pictures.receive(ctx);
            if !files.iter().any(|file| pictures.is_loading(file)) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "no answer for {files:?} in 120 s"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_picture_reaches_the_window_at_most_256_texels_on_its_longer_side() {
        let temp = scratch("pictures_longest_side");
        let export = temp.path();
        let ctx = egui::Context::default();
        let mut pictures = Pictures::default();
        // A large raster, a DDS whose 500 × 250 mip is the smallest still long enough, and a
        // small raster, which keeps its size.
        for (name, bytes, size) in [
            ("large.png", png(600, 300), [256, 128]),
            ("kit.dds", dds(1000, 500), [256, 128]),
            ("small.png", png(40, 20), [40, 20]),
        ] {
            fs::write(export.join(name), bytes).unwrap();
            let file = picture(export, name);
            assert!(pictures.texture(&ctx, &file).is_none(), "{name}: loading");
            answered(&mut pictures, &ctx, &[&file]);
            let texture = pictures.texture(&ctx, &file).expect(name);
            assert_eq!(texture.size(), size, "{name}");
        }
    }

    #[test]
    fn a_picture_that_cannot_be_read_is_asked_for_once() {
        let temp = scratch("pictures_unreadable");
        let export = temp.path();
        let ctx = egui::Context::default();
        let mut pictures = Pictures::default();
        fs::write(export.join("broken.dds"), b"not a texture").unwrap();
        let missing = picture(export, "missing.png");
        let broken = picture(export, "broken.dds");
        for file in [&missing, &broken] {
            pictures.texture(&ctx, file);
        }
        answered(&mut pictures, &ctx, &[&missing, &broken]);
        // Both become readable: asked again, the worker would now answer with a texture.
        fs::write(export.join("missing.png"), png(4, 4)).unwrap();
        fs::write(export.join("broken.dds"), dds(4, 4)).unwrap();
        for file in [&missing, &broken] {
            assert!(pictures.texture(&ctx, file).is_none(), "{file:?}");
        }
        // The worker answers in the order it is asked: once a later ask is answered, an
        // ask sent again before it would have been too.
        fs::write(export.join("later.png"), png(4, 4)).unwrap();
        let later = picture(export, "later.png");
        pictures.texture(&ctx, &later);
        answered(&mut pictures, &ctx, &[&later]);
        assert!(
            pictures.texture(&ctx, &later).is_some(),
            "premise: answered"
        );
        for file in [&missing, &broken] {
            assert!(pictures.texture(&ctx, file).is_none(), "{file:?}");
        }
    }
}
