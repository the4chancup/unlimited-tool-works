//! The selected game's portraits for the Tactics pitch: an index of the portraits the CPKs
//! `download/DpFileList.bin` lists hold (a later CPK winning), built off the UI thread, and the
//! textures of the players asked for, decoded off it. A portrait the index cannot serve gets
//! the install's own `player-dummy.png` once it is read, the suite's bundled drawing before
//! that and where the install has none.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Context;
use cpk::{CpkArchive, CpkEntry};
use crossbeam_channel::{Receiver, Sender};
use pes_savefile::portrait::portrait_path;
use pes_version::PesVersion;

/// The portrait folder's prefix in a `download` CPK (`75823.dds` below it).
const PORTRAITS: &str = "common/render/symbol/player/";

/// The game's `player-dummy.png`'s path in a CPK: what a starter whose
/// portrait cannot be shown gets once the install's copy is read.
const PLACEHOLDER: &str = "common/render/symbol/blank/player-dummy.png";

/// The suite's drawing of PES 2015's `player-dummy.png` (`resources/portraits/`).
const DUMMY_15: &[u8] = include_bytes!("../../../../../resources/portraits/player-dummy_15.png");
/// The suite's drawing of PES 2016's through 2018's `player-dummy.png` (`resources/portraits/`).
const DUMMY_16: &[u8] = include_bytes!("../../../../../resources/portraits/player-dummy_16.png");
/// The suite's drawing of PES 2019's through 2021's `player-dummy.png` (`resources/portraits/`).
const DUMMY_19: &[u8] = include_bytes!("../../../../../resources/portraits/player-dummy_19.png");

/// `version`'s bundled placeholder portrait (`resources/portraits/`): its file's name and
/// PNG bytes. The game changed its `player-dummy.png` twice, so the versions of a range share
/// one drawing.
fn placeholder_file(version: PesVersion) -> (&'static str, &'static [u8]) {
    match version {
        // PES 2014's placeholder is unmeasured; PES 2015's until it is.
        PesVersion::Pes14 | PesVersion::Pes15 => ("player-dummy_15.png", DUMMY_15),
        PesVersion::Pes16 | PesVersion::Pes17 | PesVersion::Pes18 => {
            ("player-dummy_16.png", DUMMY_16)
        }
        PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => {
            ("player-dummy_19.png", DUMMY_19)
        }
    }
}

/// `version`'s base CPK's name in the install's `Data` folder: the archive the
/// game's `player-dummy.png` ships in.
fn base_cpk(version: PesVersion) -> &'static str {
    match version {
        // PES 2014 ships a `dt15.cpk` too; whether it holds the placeholder is unmeasured.
        PesVersion::Pes14 | PesVersion::Pes15 => "dt15.cpk",
        PesVersion::Pes16 | PesVersion::Pes17 => "dt15_win.cpk",
        PesVersion::Pes18 | PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => {
            "dt15_x64.cpk"
        }
    }
}

/// The portrait source's state: the folder and the version `select` last pointed it at, its
/// worker, the players' answered textures (`resolved`), the asks still out (`pending`), the
/// bundled placeholder's uploaded texture (`placeholder`) and where the install's own
/// `player-dummy.png` stands (`game_placeholder`).
#[derive(Default)]
pub(crate) struct Portraits {
    /// The PES folder `select` last pointed the source at, `None` for "no folder": an empty
    /// setting is no folder.
    folder: Option<PathBuf>,
    /// The game `select` was last given (the open save's PES), held whether or not a folder is
    /// set or readable: the placeholder is its. `None` before the first `select`.
    version: Option<PesVersion>,
    /// The folder's reading thread, `None` when there is no folder.
    worker: Option<Worker>,
    /// The ids the worker answered: `Some`'s uploaded texture, `None` a portrait absent from
    /// the CPKs or undecodable (asked once, never again).
    resolved: HashMap<u32, Option<egui::TextureHandle>>,
    /// The ids asked of the worker, not answered yet.
    pending: HashSet<u32>,
    /// The placeholder's texture for the version it was last drawn for, decoded and uploaded
    /// once per version (`placeholder_file`).
    placeholder: Option<(PesVersion, egui::TextureHandle)>,
    /// Where the install's own `player-dummy.png` stands for this selection.
    game_placeholder: GamePlaceholder,
    /// The images the worker decoded, kept for the tests' pixel checks only.
    #[cfg(test)]
    images: HashMap<u32, egui::ColorImage>,
    /// The game's placeholder the worker decoded, kept for the tests' pixel checks only.
    #[cfg(test)]
    game_placeholder_image: Option<egui::ColorImage>,
}

/// Where the install's `player-dummy.png` stands: `NotAsked` until a portrait the worker
/// serves comes back absent or undecodable (a team usually has every portrait, so the read
/// is not done up front), `Asked` while its answer is out, `Answered` once it lands with the
/// uploaded texture (`None`: the install serves none readable, the bundled drawing keeps
/// standing in).
#[derive(Default)]
enum GamePlaceholder {
    /// No portrait came back `None` yet.
    #[default]
    NotAsked,
    /// Requested of the worker, unanswered.
    Asked,
    /// The worker's answer: the texture of the game's `player-dummy.png`, or `None`.
    Answered(Option<egui::TextureHandle>),
}

/// What a send to the worker asks for: a batch of player ids' portraits, or the game's own
/// `player-dummy.png` (`Game::placeholder`), asked once when a portrait first comes back
/// absent or undecodable.
enum Request {
    /// The portraits of these player ids.
    Ids(Vec<u32>),
    /// The install's `player-dummy.png`.
    GamePlaceholder,
}

/// The worker's answer to `Request`.
enum Reply {
    /// A player id's decoded portrait, `None` absent or undecodable.
    Image(u32, Option<egui::ColorImage>),
    /// The install's `player-dummy.png`, `None` when it holds none readable.
    GamePlaceholder(Option<egui::ColorImage>),
}

/// A selection's reading thread, as its channels: request batches in, answers out.
struct Worker {
    /// What to decode (`Request`), one batch or the one placeholder ask per send.
    requests: Sender<Request>,
    /// `Reply` per asked id or placeholder request.
    replies: Receiver<Reply>,
}

impl Worker {
    /// The worker of `folder` at `version`: it reads the index, then serves requests until the
    /// `requests` sender drops. `ctx` only gets `request_repaint` after each batch.
    fn spawn(ctx: &egui::Context, folder: &Path, version: PesVersion) -> Worker {
        let (requests, requests_rx) = crossbeam_channel::unbounded();
        let (replies, replies_rx) = crossbeam_channel::unbounded();
        let ctx = ctx.clone();
        let folder = folder.to_path_buf();
        std::thread::spawn(move || run(folder, version, requests_rx, replies, ctx));
        Worker {
            requests,
            replies: replies_rx,
        }
    }
}

impl Portraits {
    /// An empty source: no game, no portraits.
    pub(crate) fn new() -> Portraits {
        Portraits::default()
    }

    /// The PES folder `select` last pointed the source at (`None`: no folder).
    #[cfg(test)]
    pub(crate) fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// Points the source at `pes_folder` for `version` (the open save's PES and its
    /// version's PES folder setting, each frame); a change drops the current worker and
    /// its textures and starts a new one when a folder is set.
    pub(crate) fn select(&mut self, ctx: &egui::Context, pes_folder: &Path, version: PesVersion) {
        let folder = (!pes_folder.as_os_str().is_empty()).then_some(pes_folder);
        if self.folder.as_deref() == folder && self.version == Some(version) {
            return;
        }
        self.folder = folder.map(Path::to_path_buf);
        self.version = Some(version);
        self.restart(ctx);
    }

    /// The worker and its answers dropped, the source started again on the folder and
    /// version `select` last gave: an id's answered `None` is stale once the folder may
    /// hold what it lacked (a re-pick of it, or the exe check finding the game where it
    /// found none).
    pub(crate) fn restart(&mut self, ctx: &egui::Context) {
        // Dropping the request sender ends the worker; its textures go with it.
        self.worker = None;
        self.resolved.clear();
        self.pending.clear();
        self.game_placeholder = GamePlaceholder::NotAsked;
        #[cfg(test)]
        {
            self.images.clear();
            self.game_placeholder_image = None;
        }
        if let (Some(folder), Some(version)) = (&self.folder, self.version) {
            self.worker = Some(Worker::spawn(ctx, folder, version));
        }
    }

    /// The textures of `player_ids` (a roster's, `None` an empty slot), in order: a filled
    /// slot gets its decoded texture once the worker answered it, `shown_placeholder` every
    /// other time (pending, absent from the CPKs, undecodable, or no worker because no PES
    /// folder is set or the game is unreadable), so a marker never draws blank above its
    /// badge. An empty slot stays `None`, and so does every slot before the first `select`
    /// (no game, no placeholder). Asks the worker for the ids neither held nor pending,
    /// uploads what it sent back, and on the first absent or undecodable answer asks once
    /// for the install's own `player-dummy.png`.
    pub(crate) fn textures(
        &mut self,
        ctx: &egui::Context,
        player_ids: &[Option<u32>],
    ) -> Vec<Option<egui::TextureId>> {
        if let Some(worker) = &self.worker {
            let mut ask_placeholder = false;
            for reply in worker.replies.try_iter().collect::<Vec<_>>() {
                match reply {
                    Reply::Image(id, image) => {
                        self.pending.remove(&id);
                        ask_placeholder |= image.is_none();
                        #[cfg(test)]
                        if let Some(image) = &image {
                            self.images.insert(id, image.clone());
                        }
                        self.resolved.insert(
                            id,
                            image.map(|image| {
                                ctx.load_texture(
                                    format!("portrait {id}"),
                                    image,
                                    egui::TextureOptions::LINEAR,
                                )
                            }),
                        );
                    }
                    Reply::GamePlaceholder(image) => {
                        #[cfg(test)]
                        if let Some(image) = &image {
                            self.game_placeholder_image = Some(image.clone());
                        }
                        self.game_placeholder = GamePlaceholder::Answered(image.map(|image| {
                            ctx.load_texture(
                                PLACEHOLDER.to_owned(),
                                image,
                                egui::TextureOptions::LINEAR,
                            )
                        }));
                    }
                }
            }
            // The install's own `player-dummy.png` is asked for once, the first time a
            // portrait comes back absent or undecodable. The send fails only when the
            // worker is gone; `Asked` then never resolves and the drawing stands.
            if ask_placeholder && matches!(self.game_placeholder, GamePlaceholder::NotAsked) {
                self.game_placeholder = GamePlaceholder::Asked;
                if worker.requests.send(Request::GamePlaceholder).is_err() {
                    log::debug!("the portraits worker ended while a selection is held");
                }
            }
            let ask: Vec<u32> = player_ids
                .iter()
                .flatten()
                .copied()
                .filter(|id| !self.resolved.contains_key(id) && self.pending.insert(*id))
                .collect();
            // The send fails only when the worker is gone; the ids stay pending and read None.
            if !ask.is_empty() && worker.requests.send(Request::Ids(ask)).is_err() {
                log::debug!("the portraits worker ended while a selection is held");
            }
        }
        player_ids
            .iter()
            .map(|id| {
                id.and_then(|id| {
                    let held = self
                        .resolved
                        .get(&id)
                        .and_then(|held| held.as_ref())
                        .map(egui::TextureHandle::id);
                    held.or_else(|| self.shown_placeholder(ctx))
                })
            })
            .collect()
    }

    /// The texture a placeholder marker draws: the install's own
    /// `player-dummy.png` once the worker served it, `version`'s bundled
    /// drawing while its answer is out and when it came back `None` (and
    /// `None` before the first `select`).
    fn shown_placeholder(&mut self, ctx: &egui::Context) -> Option<egui::TextureId> {
        if let GamePlaceholder::Answered(Some(texture)) = &self.game_placeholder {
            return Some(texture.id());
        }
        self.version
            .map(|version| self.placeholder_texture(ctx, version))
    }

    /// `version`'s placeholder texture, decoded and uploaded the first time a version is
    /// asked for and replaced when a different one is (`placeholder_file`). A bundled file
    /// that does not decode is a build defect, so a decode failure panics naming the file
    /// rather than drawing nothing.
    fn placeholder_texture(&mut self, ctx: &egui::Context, version: PesVersion) -> egui::TextureId {
        if let Some((held, texture)) = &self.placeholder
            && *held == version
        {
            return texture.id();
        }
        let (name, _) = placeholder_file(version);
        let texture = ctx.load_texture(
            name,
            placeholder_image(version),
            egui::TextureOptions::LINEAR,
        );
        let id = texture.id();
        self.placeholder = Some((version, texture));
        id
    }

    /// `id`'s answer once the worker gave it (`Some` a decoded texture, `None` a portrait
    /// absent or undecodable); `None` while pending or never asked. The tests' wait.
    #[cfg(test)]
    pub(crate) fn answered(&self, id: u32) -> Option<Option<egui::TextureId>> {
        self.resolved
            .get(&id)
            .map(|held| held.as_ref().map(egui::TextureHandle::id))
    }

    /// The texture a placeholder marker draws: the install's own once the worker served it,
    /// else `version`'s bundled upload (`None` before `textures` first asks for it). The
    /// tests' identity check for "the marker draws the placeholder".
    #[cfg(test)]
    pub(crate) fn placeholder_id(&self, version: PesVersion) -> Option<egui::TextureId> {
        if let GamePlaceholder::Answered(Some(texture)) = &self.game_placeholder {
            return Some(texture.id());
        }
        self.placeholder
            .as_ref()
            .and_then(|(held, texture)| (*held == version).then(|| texture.id()))
    }

    /// The image the worker decoded for `id`, kept for the tests' pixel checks.
    #[cfg(test)]
    fn decoded_image(&self, id: u32) -> Option<&egui::ColorImage> {
        self.images.get(&id)
    }

    /// The game's `player-dummy.png` the worker decoded, kept for the tests' pixel checks.
    #[cfg(test)]
    fn decoded_placeholder(&self) -> Option<&egui::ColorImage> {
        self.game_placeholder_image.as_ref()
    }
}

/// `version`'s bundled placeholder portrait decoded (`placeholder_file`): what the game shows
/// for a player who has no portrait, as the suite draws it. A bundled file that does not
/// decode is a build defect, so a decode failure panics naming the file rather than drawing
/// nothing.
pub(crate) fn placeholder_image(version: PesVersion) -> egui::ColorImage {
    let (name, png) = placeholder_file(version);
    let decoded = dds_convert::decode(png, dds_convert::SourceFormat::Png).expect(name);
    let size = [decoded.width as usize, decoded.height as usize];
    egui::ColorImage::from_rgba_unmultiplied(size, decoded.mips.first().expect(name))
}

/// What `read_images` read from the folder's CPKs.
pub(crate) struct GameImages {
    /// The asked ids' decoded portraits, in order: `None` an empty slot and a player the
    /// CPKs hold no readable portrait of.
    pub(crate) portraits: Vec<Option<egui::ColorImage>>,
    /// The game's own `player-dummy.png`, read only because a filled slot's portrait came
    /// back `None`, this call's read or the one `placeholder` already held (`None` then too
    /// when the install serves none readable, or nothing needed it).
    pub(crate) placeholder: Option<egui::ColorImage>,
}

/// The portraits of `player_ids` (a roster's, `None` an empty slot) read now, on the calling
/// thread, from the CPKs of `pes_folder`, in order: a player's decoded portrait, `None` for an
/// empty slot and for a player the CPKs hold no readable portrait of. No folder (or an empty
/// setting) reads none, and so does a folder whose CPKs cannot be read, the reason logged;
/// the game's own `player-dummy.png` is read only when a filled slot came back `None` (`Game`
/// reads it once, as the worker does), and `placeholder` carries that read between calls:
/// `None` until a call first needs it and the read lands there, the answer since
/// (`Some(None)` the install serving none), so a run of pictures sharing it reads it once.
/// For a one-off read such as a picture's; the view reads through `Portraits`' worker.
pub(crate) fn read_images(
    pes_folder: Option<&Path>,
    version: PesVersion,
    player_ids: &[Option<u32>],
    placeholder: &mut Option<Option<egui::ColorImage>>,
) -> GameImages {
    let Some(folder) = pes_folder.filter(|folder| !folder.as_os_str().is_empty()) else {
        return GameImages {
            portraits: vec![None; player_ids.len()],
            placeholder: None,
        };
    };
    let mut game = Game::open(folder);
    let portraits: Vec<_> = player_ids
        .iter()
        .map(|id| id.and_then(|id| game.image(version, id)))
        .collect();
    // A starter left without a portrait draws the game's own, as the pitch shows it.
    let needed = player_ids
        .iter()
        .zip(&portraits)
        .any(|(id, image)| id.is_some() && image.is_none());
    let placeholder_image = needed
        .then(|| {
            placeholder
                .get_or_insert_with(|| game.placeholder(version))
                .clone()
        })
        .flatten();
    GameImages {
        portraits,
        placeholder: placeholder_image,
    }
}

/// The selection's worker (`Worker::spawn`): the index read, then requests served until the
/// sender drops, each answered one followed by a repaint request.
fn run(
    folder: PathBuf,
    version: PesVersion,
    requests: Receiver<Request>,
    replies: Sender<Reply>,
    ctx: egui::Context,
) {
    let mut game = Game::open(&folder);
    while let Ok(request) = requests.recv() {
        match request {
            Request::Ids(ids) => {
                for id in ids {
                    // The send fails once the selection is dropped mid-batch.
                    if replies
                        .send(Reply::Image(id, game.image(version, id)))
                        .is_err()
                    {
                        return;
                    }
                }
            }
            Request::GamePlaceholder => {
                let reply = Reply::GamePlaceholder(game.placeholder(version));
                // The send fails once the selection is dropped mid-answer.
                if replies.send(reply).is_err() {
                    return;
                }
            }
        }
        ctx.request_repaint();
    }
}

/// The game's portrait index: the CPKs `download/DpFileList.bin` lists, opened, and each
/// `PORTRAITS` or `PLACEHOLDER` entry's archive, a later CPK's entry overwriting an earlier
/// one's of the path. `folder` is kept for the base CPK's path (`placeholder`).
struct Game {
    /// The PES folder the index was read from.
    folder: PathBuf,
    /// The listed CPKs that exist, in load order.
    archives: Vec<CpkArchive<BufReader<File>>>,
    /// Each `PORTRAITS` or `PLACEHOLDER` entry's path to (its `archives` index, the entry).
    entries: HashMap<String, (usize, CpkEntry)>,
}

impl Game {
    /// `folder`'s index (`read`), or an empty one when it cannot be read, the reason logged:
    /// no portrait is shown then.
    fn open(folder: &Path) -> Game {
        Game::read(folder).unwrap_or_else(|error| {
            log::warn!("{}: no portraits: {error:#}", folder.display());
            Game {
                folder: folder.to_path_buf(),
                archives: Vec::new(),
                entries: HashMap::new(),
            }
        })
    }

    /// `folder`'s index: `download/DpFileList.bin` read, each listed CPK opened and its
    /// `PORTRAITS` and `PLACEHOLDER` entries recorded. A folder, list or CPK that cannot be
    /// read is the error:
    /// a listed CPK that is missing is one, the game itself loading none of `download/`'s CPKs
    /// then; no portrait is shown, the caller's `warn` giving the reason.
    fn read(folder: &Path) -> anyhow::Result<Game> {
        let download = folder.join("download");
        let list = download.join("DpFileList.bin");
        let bytes =
            fs::read(&list).with_context(|| format!("{}: cannot read it", list.display()))?;
        let names = cpk::dp_file_list::read(&bytes)
            .with_context(|| format!("{}: not a list the reader accepts", list.display()))?;
        let mut game = Game {
            folder: folder.to_path_buf(),
            archives: Vec::new(),
            entries: HashMap::new(),
        };
        for name in names {
            let path = download.join(&name);
            let file =
                File::open(&path).with_context(|| format!("{}: cannot open it", path.display()))?;
            let archive = CpkArchive::open(BufReader::new(file))
                .with_context(|| format!("{}: not a CPK the reader accepts", path.display()))?;
            let at = game.archives.len();
            for entry in archive.entries() {
                if entry.path.starts_with(PORTRAITS) || entry.path == PLACEHOLDER {
                    game.entries.insert(entry.path.clone(), (at, entry.clone()));
                }
            }
            game.archives.push(archive);
        }
        Ok(game)
    }

    /// `player_id`'s portrait decoded, `None` when the index has none or it does not read or
    /// decode (the reason logged).
    fn image(&mut self, version: PesVersion, player_id: u32) -> Option<egui::ColorImage> {
        let path = portrait_path(version, player_id);
        let (archive, entry) = self.entries.get(&path)?;
        let decoded = self.archives[*archive]
            .read(entry)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| Ok(dds_convert::decode(&bytes, dds_convert::SourceFormat::Dds)?));
        let decoded = match decoded {
            Ok(decoded) => decoded,
            Err(error) => {
                log::warn!("{path}: no portrait: {error:#}");
                return None;
            }
        };
        color_image(&decoded)
    }

    /// The game's own `player-dummy.png` (`PLACEHOLDER`) decoded: a listed CPK's entry when
    /// `read` recorded one (a later CPK winning), else `version`'s base CPK (`base_cpk`)
    /// under the install's `Data` folder. `None` when nothing readable holds it or it does
    /// not decode, the reason logged.
    fn placeholder(&mut self, version: PesVersion) -> Option<egui::ColorImage> {
        #[cfg(test)]
        PLACEHOLDER_READS.with(|reads| reads.set(reads.get() + 1));
        let bytes = match self.entries.get(PLACEHOLDER) {
            Some((archive, entry)) => self.archives[*archive]
                .read(entry)
                .map_err(anyhow::Error::from),
            None => self.base_placeholder(version),
        };
        let decoded = bytes
            .and_then(|bytes| Ok(dds_convert::decode(&bytes, dds_convert::SourceFormat::Png)?));
        let decoded = match decoded {
            Ok(decoded) => decoded,
            Err(error) => {
                log::warn!("{PLACEHOLDER}: no placeholder: {error:#}");
                return None;
            }
        };
        color_image(&decoded)
    }

    /// `version`'s base CPK's `player-dummy.png` bytes (`base_cpk` under the install's
    /// `Data` folder, the name every install gives it).
    fn base_placeholder(&self, version: PesVersion) -> anyhow::Result<Vec<u8>> {
        let path = self.folder.join("Data").join(base_cpk(version));
        let file =
            File::open(&path).with_context(|| format!("{}: cannot open it", path.display()))?;
        let mut archive = CpkArchive::open(BufReader::new(file))
            .with_context(|| format!("{}: not a CPK the reader accepts", path.display()))?;
        let entry = archive
            .entries()
            .iter()
            .find(|entry| entry.path == PLACEHOLDER)
            .with_context(|| format!("{}: holds no {PLACEHOLDER}", path.display()))?
            .clone();
        archive
            .read(&entry)
            .with_context(|| format!("{}: cannot read its {PLACEHOLDER}", path.display()))
    }
}

/// `decoded` as a `ColorImage`, `None` when its first mip does not fill its size.
fn color_image(decoded: &dds_convert::Decoded) -> Option<egui::ColorImage> {
    let size = [decoded.width as usize, decoded.height as usize];
    let mip = decoded.mips.first()?;
    (mip.len() == size[0] * size[1] * 4)
        .then(|| egui::ColorImage::from_rgba_unmultiplied(size, mip))
}

// `Game::placeholder` reads on the calling thread, for the tests that
// count them: thread-local so the tests' threads never count each
// other's.
#[cfg(test)]
thread_local! {
    static PLACEHOLDER_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The `Game::placeholder` reads counted so far on this thread.
#[cfg(test)]
pub(crate) fn placeholder_reads() -> usize {
    PLACEHOLDER_READS.with(|reads| reads.get())
}

/// `placeholder_reads`' zero point for a test's measured section.
#[cfg(test)]
pub(crate) fn reset_placeholder_reads() {
    PLACEHOLDER_READS.with(|reads| reads.set(0));
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::test_support::{
        base_cpk as write_base_cpk, dp_file_list, dummy_png, pes_folder, scratch,
    };

    /// PES 2021's `75823.dds` in `4cc_40_faces.cpk`, pixel (64, 64) `(148, 219, 247, 255)`.
    const FACES: &[u8] = include_bytes!("../../tests/fixtures/portraits/75823_faces.dds");
    /// The same path in `4cc_66_midcup.cpk`, pixel (64, 64) `(255, 255, 255, 255)`.
    const MIDCUP: &[u8] = include_bytes!("../../tests/fixtures/portraits/75823_midcup66.dds");
    /// PES 2017's `player_71801.dds`, WESYS-wrapped as the game ships it.
    const PES17: &[u8] = include_bytes!("../../tests/fixtures/portraits/player_71801_pes17.dds");

    /// `textures` polled until every asked id's answer lands, a real nap between polls: the
    /// bound makes a worker that never answers a failure, not a hang. A hang guard, not a
    /// speed bound: a passing test returns when the answer lands, and a 2 s bound failed
    /// two of these on a machine busy with three builds (2026-10-29), the worker answering
    /// in milliseconds once the load was gone.
    fn served(
        portraits: &mut Portraits,
        ctx: &egui::Context,
        ids: &[Option<u32>],
    ) -> Vec<Option<egui::TextureId>> {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let answer = portraits.textures(ctx, ids);
            if ids
                .iter()
                .flatten()
                .all(|id| portraits.answered(*id).is_some())
            {
                return answer;
            }
            assert!(Instant::now() < deadline, "no answer for {ids:?} in 120 s");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// `textures` polled until the game's placeholder answer lands, the same bound as
    /// `served`: `Answered` is the state a reply moves `game_placeholder` to.
    fn dummy_served(portraits: &mut Portraits, ctx: &egui::Context, ids: &[Option<u32>]) {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            portraits.textures(ctx, ids);
            if matches!(portraits.game_placeholder, GamePlaceholder::Answered(_)) {
                return;
            }
            assert!(Instant::now() < deadline, "no placeholder answer in 120 s");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// `pixel` within 2 of `expected` in every channel (BC rounding differs between decoders).
    fn close(pixel: egui::Color32, expected: [u8; 4]) {
        for (channel, expected) in pixel.to_array().into_iter().zip(expected) {
            assert!(
                channel.abs_diff(expected) <= 2,
                "{pixel:?} within 2 of {expected:?}"
            );
        }
    }

    // The pitch's portrait source: the CPKs the selected game's `download/DpFileList.bin`
    // lists, a later one's portrait winning, as in game.
    #[test]
    fn a_later_listed_cpk_s_portrait_wins() {
        let ctx = egui::Context::default();
        let path = portrait_path(PesVersion::Pes21, 75823);
        for (case, names, expected) in [
            (
                "midcup_later",
                ["4cc_40_faces.cpk", "4cc_66_midcup.cpk"],
                [255, 255, 255, 255],
            ),
            (
                "faces_later",
                ["4cc_66_midcup.cpk", "4cc_40_faces.cpk"],
                [148, 219, 247, 255],
            ),
        ] {
            let bytes = |name: &str| {
                if name.contains("faces") {
                    FACES
                } else {
                    MIDCUP
                }
            };
            let first: &[(&str, &[u8])] = &[(path.as_str(), bytes(names[0]))];
            let second: &[(&str, &[u8])] = &[(path.as_str(), bytes(names[1]))];
            let cpks = [(names[0], first), (names[1], second)];
            let folder = pes_folder(&format!("portraits_{case}"), &cpks);
            let mut portraits = Portraits::new();
            portraits.select(&ctx, &folder, PesVersion::Pes21);

            assert!(
                served(&mut portraits, &ctx, &[Some(75823)])[0].is_some(),
                "{case}: a texture for 75823"
            );
            let image = portraits.decoded_image(75823).expect("decoded");
            assert_eq!(image.size, [128, 128], "{case}");
            close(image.pixels[64 * 128 + 64], expected);
        }
    }

    // The one-off read a picture makes, on the calling thread: a held portrait decoded, none
    // for a player the CPKs lack or an empty slot, and none at all without a folder.
    #[test]
    fn a_one_off_read_decodes_the_held_portraits() {
        let path = portrait_path(PesVersion::Pes21, 75823);
        let folder = pes_folder(
            "portraits_one_off",
            &[("4cc_40_faces.cpk", &[(path.as_str(), FACES)])],
        );
        let ids = [Some(75823), Some(75824), None];
        let images = read_images(Some(&folder), PesVersion::Pes21, &ids, &mut None);
        let held = images.portraits[0].as_ref().expect("75823's portrait");
        close(held.pixels[64 * 128 + 64], [148, 219, 247, 255]);
        assert!(images.portraits[1].is_none(), "75824 has no portrait");
        assert!(images.portraits[2].is_none(), "an empty slot");
        assert!(
            read_images(None, PesVersion::Pes21, &ids, &mut None)
                .portraits
                .iter()
                .all(Option::is_none),
            "no folder"
        );
    }

    // The game's placeholder is read only when a filled slot's portrait is missing: an
    // empty slot alone does not ask for it, and the read lands in the cache passed in.
    #[test]
    fn the_game_s_placeholder_is_read_only_for_a_missing_portrait() {
        let path = portrait_path(PesVersion::Pes21, 75823);
        let folder = pes_folder(
            "portraits_placeholder_needed",
            &[("4cc_40_faces.cpk", &[(path.as_str(), FACES)])],
        );
        let mut cache = None;
        let ids = [Some(75823), None];
        read_images(Some(&folder), PesVersion::Pes21, &ids, &mut cache);
        assert!(
            cache.is_none(),
            "every filled slot holding its portrait: the placeholder is not read"
        );

        let ids = [Some(75823), Some(75824), None];
        read_images(Some(&folder), PesVersion::Pes21, &ids, &mut cache);
        assert!(
            cache.is_some(),
            "75824's missing portrait read the placeholder into the cache"
        );
    }

    // The same source on a pre-Fox portrait: `player_71801.dds`, WESYS-wrapped as the game
    // ships it, decoded at the size its header gives.
    #[test]
    fn a_pes_17_portrait_unwraps_wesys_and_decodes() {
        let ctx = egui::Context::default();
        let path = portrait_path(PesVersion::Pes17, 71801);
        assert_eq!(path, "common/render/symbol/player/player_71801.dds");
        let folder = pes_folder(
            "portraits_pes17",
            &[("4cc_40_faces.cpk", &[(path.as_str(), PES17)])],
        );
        let header = dds_convert::probe(PES17, dds_convert::SourceFormat::Dds).unwrap();
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes17);

        assert!(served(&mut portraits, &ctx, &[Some(71801)])[0].is_some());
        let image = portraits.decoded_image(71801).expect("decoded");
        assert_eq!(
            image.size,
            [header.width as usize, header.height as usize],
            "the header's size"
        );
    }

    // A folder without a `download/` and a list naming a CPK that is not there: no panic,
    // every id answered (None, so the placeholder) within the bound, an empty slot left out.
    #[test]
    fn an_unreadable_game_and_a_missing_cpk_draw_the_placeholder() {
        let ctx = egui::Context::default();
        let ids = [None, Some(75823), Some(1)];

        // No `download/` at all.
        let folder = scratch("portraits_no_download").join("PES");
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes21);
        let shown = served(&mut portraits, &ctx, &ids);
        let placeholder = portraits
            .placeholder_id(PesVersion::Pes21)
            .expect("the placeholder uploaded");
        assert_eq!(shown, [None, Some(placeholder), Some(placeholder)]);

        // The list names a CPK that is absent: the game loads none of
        // `download/` then, so a portrait the other CPK holds is not
        // served either.
        let path = portrait_path(PesVersion::Pes21, 75823);
        let download = scratch("portraits_missing_cpk").join("PES/download");
        fs::create_dir_all(&download).unwrap();
        fs::write(
            download.join("DpFileList.bin"),
            dp_file_list(&["4cc_40_faces.cpk", "4cc_66_midcup.cpk"]),
        )
        .unwrap();
        crate::test_support::write_cpk(
            &download.join("4cc_40_faces.cpk"),
            &[(path.as_str(), FACES)],
        );
        let folder = download.parent().unwrap().to_path_buf();
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes21);
        let shown = served(&mut portraits, &ctx, &ids);
        let placeholder = portraits
            .placeholder_id(PesVersion::Pes21)
            .expect("the placeholder uploaded");
        assert_eq!(shown, [None, Some(placeholder), Some(placeholder)]);
    }

    // The placeholder is the selected game's (the version `select` was last given, the
    // sidebar's), not the open save's: a source selected for PES 2017 with
    // an unreadable folder, then with none set, draws PES 2017's placeholder for a PES 2021
    // save's roster, never PES 2021's.
    #[test]
    fn the_placeholder_is_the_selected_games() {
        let ctx = egui::Context::default();
        let ids = [None, Some(75823)];

        let folder = scratch("portraits_selected_game").join("PES");
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes17);
        let shown = served(&mut portraits, &ctx, &ids);
        let placeholder = portraits
            .placeholder_id(PesVersion::Pes17)
            .expect("PES 2017's placeholder uploaded");
        assert_eq!(shown, [None, Some(placeholder)], "an unreadable folder");
        assert_eq!(portraits.placeholder_id(PesVersion::Pes21), None);

        let mut portraits = Portraits::new();
        portraits.select(&ctx, Path::new(""), PesVersion::Pes17);
        let shown = portraits.textures(&ctx, &ids);
        let placeholder = portraits
            .placeholder_id(PesVersion::Pes17)
            .expect("PES 2017's placeholder uploaded");
        assert_eq!(shown, [None, Some(placeholder)], "no folder set");
        assert_eq!(portraits.placeholder_id(PesVersion::Pes21), None);
    }

    // A change of the PES folder or the version drops the worker and every answered texture
    // (the new selection starts empty); the same selection keeps them.
    #[test]
    fn a_changed_selection_starts_over_and_the_same_one_keeps_its_portraits() {
        let ctx = egui::Context::default();
        let path = portrait_path(PesVersion::Pes21, 75823);
        let faces: &[(&str, &[u8])] = &[(path.as_str(), FACES)];
        let midcup: &[(&str, &[u8])] = &[(path.as_str(), MIDCUP)];
        let folder_a = pes_folder("portraits_select_a", &[("4cc_40_faces.cpk", faces)]);
        let folder_b = pes_folder("portraits_select_b", &[("4cc_66_midcup.cpk", midcup)]);
        let mut portraits = Portraits::new();

        portraits.select(&ctx, &folder_a, PesVersion::Pes21);
        assert!(served(&mut portraits, &ctx, &[Some(75823)])[0].is_some());
        close(
            portraits.decoded_image(75823).expect("decoded").pixels[64 * 128 + 64],
            [148, 219, 247, 255],
        );

        // The same selection: the answer is held, nothing re-asked or cleared.
        portraits.select(&ctx, &folder_a, PesVersion::Pes21);
        assert!(
            matches!(portraits.answered(75823), Some(Some(_))),
            "the same selection keeps the texture"
        );

        // Another folder: cleared at once, then the new game's portrait.
        portraits.select(&ctx, &folder_b, PesVersion::Pes21);
        assert_eq!(portraits.answered(75823), None, "a new folder clears");
        assert!(served(&mut portraits, &ctx, &[Some(75823)])[0].is_some());
        close(
            portraits.decoded_image(75823).expect("decoded").pixels[64 * 128 + 64],
            [255, 255, 255, 255],
        );

        // The same folder, another version (the same `<id>.dds` path): cleared again.
        portraits.select(&ctx, &folder_b, PesVersion::Pes20);
        assert_eq!(portraits.answered(75823), None, "a version change clears");
        assert!(served(&mut portraits, &ctx, &[Some(75823)])[0].is_some());
        close(
            portraits.decoded_image(75823).expect("decoded").pixels[64 * 128 + 64],
            [255, 255, 255, 255],
        );
    }

    // An asked id sits in `pending` until its answer lands and is asked once: `pending` and
    // `resolved` are the two gates a request goes through. The worker here is a pair of
    // test-owned channels, not a thread: a request or an answer lands exactly when the test
    // says, so the gates are checked with no wait at all.
    #[test]
    fn an_asked_id_is_pending_until_answered() {
        let ctx = egui::Context::default();
        let (requests, requests_rx) = crossbeam_channel::unbounded();
        let (replies, replies_rx) = crossbeam_channel::unbounded();
        let mut portraits = Portraits {
            version: Some(PesVersion::Pes21),
            worker: Some(Worker {
                requests,
                replies: replies_rx,
            }),
            ..Portraits::new()
        };

        // The ask: pending, not resolved, one request on the channel, the slot drawing the
        // placeholder until the answer lands.
        assert_eq!(
            portraits.textures(&ctx, &[Some(75823)]),
            [Some(
                portraits
                    .placeholder_id(PesVersion::Pes21)
                    .expect("the placeholder uploaded")
            )]
        );
        assert!(portraits.pending.contains(&75823), "asked and pending");
        assert!(!portraits.resolved.contains_key(&75823));
        assert!(
            matches!(requests_rx.try_recv(), Ok(Request::Ids(ids)) if ids == [75823]),
            "the one request"
        );

        // Still pending: the same call sends nothing again.
        portraits.textures(&ctx, &[Some(75823)]);
        assert!(
            matches!(
                requests_rx.try_recv(),
                Err(crossbeam_channel::TryRecvError::Empty)
            ),
            "no second request for a pending id"
        );

        // The answer: resolved, pending emptied, and still nothing re-asked.
        replies
            .send(Reply::Image(
                75823,
                Some(egui::ColorImage::filled([1, 1], egui::Color32::RED)),
            ))
            .unwrap();
        assert!(served(&mut portraits, &ctx, &[Some(75823)])[0].is_some());
        assert!(portraits.pending.is_empty(), "answered, pending emptied");
        portraits.textures(&ctx, &[Some(75823)]);
        assert!(
            matches!(
                requests_rx.try_recv(),
                Err(crossbeam_channel::TryRecvError::Empty)
            ),
            "no request for a held texture"
        );
    }

    // The install's `player-dummy.png` is asked for once: a batch of
    // answers holding an absent portrait sends one `GamePlaceholder`
    // request, and a later absent portrait sends none, the ask standing
    // in `Asked`. The worker is a pair of test-owned channels, as in
    // `an_asked_id_is_pending_until_answered`, so a request or an answer
    // lands exactly when the test says.
    #[test]
    fn the_games_dummy_is_asked_once() {
        let ctx = egui::Context::default();
        let (requests, requests_rx) = crossbeam_channel::unbounded();
        let (replies, replies_rx) = crossbeam_channel::unbounded();
        let mut portraits = Portraits {
            version: Some(PesVersion::Pes21),
            worker: Some(Worker {
                requests,
                replies: replies_rx,
            }),
            ..Portraits::new()
        };

        // A batch holding one absent portrait (id 1): one ask, not one per
        // answer.
        replies.send(Reply::Image(1, None)).unwrap();
        replies
            .send(Reply::Image(
                2,
                Some(egui::ColorImage::filled([1, 1], egui::Color32::RED)),
            ))
            .unwrap();
        portraits.textures(&ctx, &[]);
        assert!(
            matches!(requests_rx.try_recv(), Ok(Request::GamePlaceholder)),
            "the first absent portrait asks for the game's dummy"
        );
        assert!(
            matches!(
                requests_rx.try_recv(),
                Err(crossbeam_channel::TryRecvError::Empty)
            ),
            "exactly once for the batch"
        );

        // `Asked` now: a later absent portrait asks nothing again.
        replies.send(Reply::Image(3, None)).unwrap();
        portraits.textures(&ctx, &[]);
        assert!(
            matches!(
                requests_rx.try_recv(),
                Err(crossbeam_channel::TryRecvError::Empty)
            ),
            "no second ask while the first is out"
        );
    }

    // Each version's bundled placeholder decodes at the game's size: 2015's file for PES
    // 2015, the 2016 one for 2016 through 2018, the 2019 one for 2019 through 2021
    // (`resources/portraits/`; the centre pixels measured with Pillow).
    #[test]
    fn each_version_s_placeholder_decodes_at_128() {
        for (version, centre) in [
            (PesVersion::Pes15, [236, 240, 243, 255]),
            (PesVersion::Pes16, [146, 156, 172, 255]),
            (PesVersion::Pes17, [146, 156, 172, 255]),
            (PesVersion::Pes18, [146, 156, 172, 255]),
            (PesVersion::Pes19, [204, 192, 184, 255]),
            (PesVersion::Pes20, [204, 192, 184, 255]),
            (PesVersion::Pes21, [204, 192, 184, 255]),
        ] {
            let decoded =
                dds_convert::decode(placeholder_file(version).1, dds_convert::SourceFormat::Png)
                    .unwrap_or_else(|error| panic!("{version:?}: {error}"));
            assert_eq!(decoded.width, 128, "{version:?}");
            assert_eq!(decoded.height, 128, "{version:?}");
            let mip = &decoded.mips[0];
            assert_eq!(mip.len(), 128 * 128 * 4, "{version:?}");
            assert_eq!(mip[(64 * 128 + 64) * 4..][..4], centre, "{version:?}");
        }
    }

    // A starter whose portrait the pitch cannot show draws the game's placeholder: one whose
    // portrait is absent from the CPKs or does not decode (each answered `None`), and every
    // filled slot when there is no worker at all because no PES folder is set. An empty slot
    // stays `None`.
    #[test]
    fn an_unshown_portrait_draws_the_placeholder() {
        let ctx = egui::Context::default();
        let ids = [None, Some(75823), Some(1)];

        // 75823's entry holds bytes that are no DDS; 1 has no entry at all.
        let path = portrait_path(PesVersion::Pes21, 75823);
        let bad: &[(&str, &[u8])] = &[(path.as_str(), b"not a dds")];
        let folder = pes_folder("portraits_placeholder", &[("4cc_40_faces.cpk", bad)]);
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes21);
        let shown = served(&mut portraits, &ctx, &ids);
        let placeholder = portraits
            .placeholder_id(PesVersion::Pes21)
            .expect("the placeholder uploaded");
        assert_eq!(shown, [None, Some(placeholder), Some(placeholder)]);

        // No PES folder set: no worker, so no ask ever resolves; every filled slot draws
        // the placeholder from the first call, no wait needed.
        let mut portraits = Portraits::new();
        portraits.select(&ctx, Path::new(""), PesVersion::Pes21);
        let shown = portraits.textures(&ctx, &ids);
        let placeholder = portraits
            .placeholder_id(PesVersion::Pes21)
            .expect("the placeholder uploaded");
        assert_eq!(shown, [None, Some(placeholder), Some(placeholder)]);
    }

    // A starter whose portrait is in the CPKs gets it, not the placeholder: asked together
    // with an id the CPK lacks, the two slots' textures differ and the real one decodes to
    // the fixture's pixels.
    #[test]
    fn a_shown_portrait_is_not_the_placeholder() {
        let ctx = egui::Context::default();
        let path = portrait_path(PesVersion::Pes21, 75823);
        let faces: &[(&str, &[u8])] = &[(path.as_str(), FACES)];
        let folder = pes_folder("portraits_real", &[("4cc_40_faces.cpk", faces)]);
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes21);

        let shown = served(&mut portraits, &ctx, &[Some(75823), Some(1)]);
        let placeholder = portraits
            .placeholder_id(PesVersion::Pes21)
            .expect("the placeholder uploaded");
        assert_eq!(shown[1], Some(placeholder), "the absent id's slot");
        let portrait = shown[0].expect("a texture for 75823");
        assert_ne!(
            portrait, placeholder,
            "the real portrait, not the placeholder"
        );
        close(
            portraits.decoded_image(75823).expect("decoded").pixels[64 * 128 + 64],
            [148, 219, 247, 255],
        );
    }

    // The placeholder is decoded and uploaded once per version: another version swaps in its
    // own file's texture (PES 2017's the `_16` file's pixels), asking for one version twice
    // keeps the texture.
    #[test]
    fn the_placeholder_is_decoded_once_per_version() {
        let ctx = egui::Context::default();
        // No folder, so no worker: `textures` answers the placeholder at once for any filled
        // slot.
        let mut portraits = Portraits::new();
        let absent = [Some(1)];

        portraits.select(&ctx, Path::new(""), PesVersion::Pes21);
        let first = portraits.textures(&ctx, &absent)[0].expect("shown");
        portraits.select(&ctx, Path::new(""), PesVersion::Pes17);
        let second = portraits.textures(&ctx, &absent)[0].expect("shown");
        assert_ne!(first, second, "another version uploads its own placeholder");
        let decoded = dds_convert::decode(
            placeholder_file(PesVersion::Pes17).1,
            dds_convert::SourceFormat::Png,
        )
        .expect("the bundled PNG decodes");
        assert_eq!(
            decoded.mips[0][(64 * 128 + 64) * 4..][..4],
            [146, 156, 172, 255],
            "PES 2017's is the _16 file"
        );
        let again = portraits.textures(&ctx, &absent)[0].expect("shown");
        assert_eq!(again, second, "the same version is not decoded again");
    }

    // A starter whose portrait no CPK holds gets the game's own `player-dummy.png` once it
    // is read: here the base CPK's (`Data/dt15_x64.cpk` for PES 2021). The marker's texture
    // is the install's image, checked by pixel through the decoded image, not the bundled
    // drawing's.
    #[test]
    fn a_portraitless_starter_draws_the_games_dummy() {
        let ctx = egui::Context::default();
        let colour = [255, 0, 255, 255];
        let dummy = dummy_png(colour);
        let folder = pes_folder("portraits_games_dummy", &[]);
        write_base_cpk(&folder, "dt15_x64.cpk", &[(PLACEHOLDER, dummy.as_slice())]);
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes21);
        assert_ne!(
            placeholder_image(PesVersion::Pes21).pixels[64 * 128 + 64].to_array(),
            colour,
            "premise: the bundled drawing's centre differs"
        );

        assert!(served(&mut portraits, &ctx, &[Some(75823)])[0].is_some());
        dummy_served(&mut portraits, &ctx, &[Some(75823)]);
        assert_eq!(
            portraits.textures(&ctx, &[Some(75823)])[0],
            portraits.placeholder_id(PesVersion::Pes21),
            "the marker draws the game's dummy"
        );
        let image = portraits.decoded_placeholder().expect("decoded");
        assert_eq!(image.size, [128, 128]);
        close(image.pixels[64 * 128 + 64], colour);
    }

    // A listed `download` CPK's `player-dummy.png` wins over the base CPK's, as for portraits.
    #[test]
    fn a_listed_cpks_dummy_wins_over_the_base_cpks() {
        let ctx = egui::Context::default();
        let listed = dummy_png([255, 0, 255, 255]);
        let base = dummy_png([0, 255, 255, 255]);
        let folder = pes_folder(
            "portraits_dummy_wins",
            &[("4cc_40_faces.cpk", &[(PLACEHOLDER, listed.as_slice())])],
        );
        write_base_cpk(&folder, "dt15_x64.cpk", &[(PLACEHOLDER, base.as_slice())]);
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes21);

        assert!(served(&mut portraits, &ctx, &[Some(75823)])[0].is_some());
        dummy_served(&mut portraits, &ctx, &[Some(75823)]);
        let image = portraits.decoded_placeholder().expect("decoded");
        close(image.pixels[64 * 128 + 64], [255, 0, 255, 255]);
    }

    // A team whose every starter has a portrait never asks for the game's dummy:
    // `game_placeholder` stays `NotAsked` (a request would move it to `Asked`, then
    // `Answered`). The request is the only path to the base CPK, whose bytes here are no
    // CPK at all.
    #[test]
    fn a_team_of_portraits_never_reads_the_base_cpk() {
        let ctx = egui::Context::default();
        let path = portrait_path(PesVersion::Pes21, 75823);
        let faces: &[(&str, &[u8])] = &[(path.as_str(), FACES)];
        let folder = pes_folder("portraits_no_dummy_read", &[("4cc_40_faces.cpk", faces)]);
        let data = folder.join("Data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("dt15_x64.cpk"), b"not a cpk").unwrap();
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes21);

        assert!(served(&mut portraits, &ctx, &[Some(75823)])[0].is_some());
        assert!(
            matches!(portraits.game_placeholder, GamePlaceholder::NotAsked),
            "no placeholder request: the base CPK was never opened"
        );
    }

    // No `Data` folder in the install: the placeholder request answers `None` and the
    // marker keeps the bundled drawing.
    #[test]
    fn no_data_folder_keeps_the_bundled_drawing() {
        let ctx = egui::Context::default();
        let folder = pes_folder("portraits_no_data", &[]);
        let mut portraits = Portraits::new();
        portraits.select(&ctx, &folder, PesVersion::Pes21);

        assert!(served(&mut portraits, &ctx, &[Some(75823)])[0].is_some());
        dummy_served(&mut portraits, &ctx, &[Some(75823)]);
        assert!(
            matches!(portraits.game_placeholder, GamePlaceholder::Answered(None)),
            "the install serves none"
        );
        assert!(portraits.decoded_placeholder().is_none());
        assert_eq!(
            portraits.textures(&ctx, &[Some(75823)])[0],
            portraits.placeholder_id(PesVersion::Pes21),
            "the bundled drawing, as before"
        );
    }
}
