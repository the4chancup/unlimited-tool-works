//! The PES 2014 Gameplan's tooltips: the game's own help texts, read once per
//! PES folder and save version — selected and restarted as the portraits' source
//! is — off the UI thread, from the English language archive's `all.str`
//! (`ENGLISH_ARCHIVE`'s `ALL_STR` entry). With no folder set, or one that cannot
//! be read, the tooltips are absent; no frame's code touches the disk.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Context;
use cpk::CpkArchive;
use crossbeam_channel::{Receiver, TryRecvError};
use pes_version::PesVersion;
use team_widgets::tactics::pes14::{self, Tips};

/// The English language archive inside a PES 2014 install.
const ENGLISH_ARCHIVE: &str = "Data/dt21_eng.cpk";
/// The texts' entry inside it.
const ALL_STR: &str = "pes14_win_dat/eng/string/all.str";

/// The Gameplan tooltips' source: the folder and the version `select` last
/// pointed it at, the read in flight (`pending`) and the resolved texts
/// (`tips`, `None` until one lands and after one failed).
#[derive(Default)]
pub(crate) struct GameTips {
    /// The PES folder `select` last pointed the source at, `None` for "no
    /// folder": an empty setting is no folder.
    folder: Option<PathBuf>,
    /// The game `select` was last given: only a PES 2014 save's screen reads
    /// the texts.
    version: Option<PesVersion>,
    /// The read in flight's reply (`Some` the resolved tooltips, `None` a
    /// failed read), `None` idle.
    pending: Option<Receiver<Option<Tips>>>,
    /// The resolved tooltips of `folder` at `version`.
    tips: Option<Tips>,
}

impl GameTips {
    /// An empty source: no game, no tooltips.
    pub(crate) fn new() -> GameTips {
        GameTips::default()
    }

    /// Points the source at `pes_folder` for `version` (the open save's PES and
    /// its version's PES folder setting, each frame); a change drops the
    /// in-flight read and the held texts and starts a new one when a folder is
    /// set and the save is PES 2014's.
    pub(crate) fn select(&mut self, ctx: &egui::Context, pes_folder: &Path, version: PesVersion) {
        let folder = (!pes_folder.as_os_str().is_empty()).then_some(pes_folder);
        if self.folder.as_deref() == folder && self.version == Some(version) {
            return;
        }
        self.folder = folder.map(Path::to_path_buf);
        self.version = Some(version);
        self.restart(ctx);
    }

    /// The in-flight read and the held texts dropped, the source started again
    /// on the folder and version `select` last gave: a re-picked or repaired
    /// folder may hold what the last read lacked.
    pub(crate) fn restart(&mut self, ctx: &egui::Context) {
        self.pending = None;
        self.tips = None;
        if self.version != Some(PesVersion::Pes14) {
            return;
        }
        if let Some(folder) = self.folder.clone() {
            let (sender, reply) = crossbeam_channel::unbounded();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                // The send fails only once the selection moved on.
                if sender.send(read(&folder)).is_ok() {
                    ctx.request_repaint();
                }
            });
            self.pending = Some(reply);
        }
    }

    /// The resolved tooltips, `None` while the read runs, after one failed and
    /// on a save that is not PES 2014's. Drains the reply first, so a frame's
    /// call sees the newest state.
    pub(crate) fn tips(&mut self) -> Option<&Tips> {
        self.poll();
        self.tips.as_ref()
    }

    /// Whether no read is in flight (the tests' wait).
    #[cfg(test)]
    pub(crate) fn settled(&mut self) -> bool {
        self.poll();
        self.pending.is_none()
    }

    /// The in-flight read's reply taken, if it landed.
    fn poll(&mut self) {
        if let Some(pending) = &self.pending {
            match pending.try_recv() {
                Ok(tips) => {
                    self.tips = tips;
                    self.pending = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.pending = None,
            }
        }
    }
}

/// `folder`'s tooltips (`read_all_str` then `resolve`), `None` on any failure,
/// its reason logged once — a missing archive, a missing entry, a read error —
/// the tooltips are absent then, never an error to surface.
fn read(folder: &Path) -> Option<Tips> {
    match read_all_str(folder) {
        Ok(file) => Some(resolve(&file)),
        Err(error) => {
            log::warn!("{}: no Gameplan tooltips: {error:#}", folder.display());
            None
        }
    }
}

/// `folder`'s `all.str`: `ENGLISH_ARCHIVE`'s `ALL_STR` entry, opened and read.
fn read_all_str(folder: &Path) -> anyhow::Result<pes_strings::StrFile> {
    let path = folder.join(ENGLISH_ARCHIVE);
    let file = File::open(&path).with_context(|| format!("{}: cannot open it", path.display()))?;
    let mut archive = CpkArchive::open(BufReader::new(file))
        .with_context(|| format!("{}: not a CPK the reader accepts", path.display()))?;
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path == ALL_STR)
        .with_context(|| format!("{}: no {ALL_STR} entry", path.display()))?
        .clone();
    let bytes = archive
        .read(&entry)
        .with_context(|| format!("{}'s {ALL_STR}: cannot read it", path.display()))?;
    pes_strings::read(&bytes).with_context(|| {
        format!(
            "{}'s {ALL_STR}: not an all.str the reader accepts",
            path.display()
        )
    })
}

/// `file`'s texts resolved into the tooltips: each of `pes14`'s `*_TIPS` tables'
/// string looked up and cleaned (`clean`), the Automatic settings' shared text
/// cut at each setting's heading (`block`), `None` where the file lacks it.
fn resolve(file: &pes_strings::StrFile) -> Tips {
    let text = |tip: (&'static str, u32)| {
        file.section(tip.0)
            .and_then(|section| section.text(tip.1))
            .map(clean)
            .filter(|text| !text.is_empty())
    };
    let block = |heading: &'static str| {
        file.section(pes14::AUTO_TIP.0)
            .and_then(|section| section.text(pes14::AUTO_TIP.1))
            .and_then(|text| block(text, heading))
    };
    let mut tips = Tips::default();
    for (held, tip) in tips.sliders.iter_mut().zip(pes14::SLIDER_TIPS) {
        *held = text(tip);
    }
    for (held, tip) in tips.strategies.iter_mut().zip(pes14::STRATEGY_TIPS) {
        *held = text(tip);
    }
    for (held, tip) in tips.formations.iter_mut().zip(pes14::FORMATION_TIPS) {
        *held = text(tip);
    }
    for (held, tip) in tips.set_pieces.iter_mut().zip(&pes14::SET_PIECE_TIPS[..6]) {
        *held = text(*tip);
    }
    tips.join_attack = text(pes14::SET_PIECE_TIPS[6]);
    tips.substitution = block(pes14::SUBSTITUTION_BLOCK);
    tips.preset_change = block(pes14::PRESET_CHANGE_BLOCK);
    tips.offside_trap = block(pes14::OFFSIDE_TRAP_BLOCK);
    tips.combination_play = block(pes14::COMBINATION_PLAY_BLOCK);
    tips.combination_title = text(pes14::COMBINATION_TITLE_TIP);
    tips
}

/// `text` as a tooltip shows it: the game's emphasis marks (`\u{11}i1` on,
/// `\u{11}i0` off) dropped, a tooltip drawing none, and the one button code the
/// tooltips' texts carry (`\u{11}ll`, the left trigger) written "LT", as the
/// game draws it. The texts' own line breaks are kept.
fn clean(text: &str) -> String {
    text.replace("\u{11}i1", "")
        .replace("\u{11}i0", "")
        .replace("\u{11}ll", "LT")
}

/// The block headed `<heading>` in `text` (the Automatic settings' shared
/// `F3T` 93): the text after it up to the next heading or the end, cleaned as a
/// tooltip shows it, the line breaks around it trimmed. `None` when `text`
/// holds no such heading, or the block is empty — a tooltip that has no text is
/// no tooltip.
fn block(text: &str, heading: &str) -> Option<String> {
    let after = text.split_once(&format!("<{heading}>"))?.1;
    let end = after.find('<').unwrap_or(after.len());
    let block = clean(&after[..end]).trim_matches(['\r', '\n']).to_owned();
    (!block.is_empty()).then_some(block)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::test_support::{scratch, write_cpk};

    /// PES 2014's own `all.str` (`pes_strings`' fixture).
    const ALL_STR_FILE: &[u8] =
        include_bytes!("../../../../libs/pes_strings/tests/fixtures/pes14_all.str");

    /// `test`'s scratch holding a PES folder whose `Data/dt21_eng.cpk` has the
    /// fixture's `all.str` entry.
    fn folder_with_all_str(test: &str) -> PathBuf {
        let folder = scratch(test).join("PES");
        write_cpk(&folder.join(ENGLISH_ARCHIVE), &[(ALL_STR, ALL_STR_FILE)]);
        folder
    }

    /// `tips`' read polled until it lands or fails (bounded: a worker that
    /// never answers fails rather than hangs; a hang guard, not a speed bound,
    /// the portraits tests' 120 s).
    fn settled_tips(tips: &mut GameTips) -> Option<&Tips> {
        let deadline = Instant::now() + Duration::from_secs(120);
        while !tips.settled() {
            assert!(Instant::now() < deadline, "the read never settled in 120 s");
            std::thread::sleep(Duration::from_millis(10));
        }
        tips.tips()
    }

    // Every table entry resolves to a text in PES 2014's own file.
    #[test]
    fn the_fixture_resolves_every_tooltip() {
        let file = pes_strings::read(ALL_STR_FILE).unwrap();
        let tips = resolve(&file);
        assert!(tips.sliders.iter().all(Option::is_some), "sliders");
        assert!(tips.strategies.iter().all(Option::is_some), "strategies");
        assert!(tips.formations.iter().all(Option::is_some), "formations");
        assert!(tips.set_pieces.iter().all(Option::is_some), "set pieces");
        for tip in [
            &tips.join_attack,
            &tips.substitution,
            &tips.preset_change,
            &tips.offside_trap,
            &tips.combination_play,
            &tips.combination_title,
        ] {
            assert!(tip.is_some(), "a named tooltip");
        }
    }

    // The texts as the tooltips show them, word for word from the game's file:
    // a plain entry, a block of the shared text cut at its heading, the button
    // code written LT with no inline code left, and a text's own line break
    // kept.
    #[test]
    fn the_fixture_s_texts_are_the_games_words() {
        let file = pes_strings::read(ALL_STR_FILE).unwrap();
        let tips = resolve(&file);
        assert_eq!(
            tips.strategies[1].as_deref(),
            Some(
                "Play a tight backline that can win back possession and play long balls to launch counter attacks."
            ),
            "Long Ball's text"
        );
        assert_eq!(
            tips.offside_trap.as_deref(),
            Some(
                "Choose whether or not players will automatically attempt to play offside traps during matches."
            ),
            "Offside trap's block of the shared text"
        );
        let title = tips.combination_title.as_deref().expect("the title's text");
        assert!(
            title.ends_with("press LT twice when the Combination Play icon is shown on screen."),
            "the button code written LT: {title:?}"
        );
        assert!(!title.contains('\u{11}'), "no inline code left: {title:?}");
        assert_eq!(
            tips.join_attack.as_deref(),
            Some(
                "Select which defenders will go forward and join the attack in Set Pieces.\nUp to 3 defenders can go up front. If you choose a 4th, the defender who was first selected will elect to stay back in his place."
            ),
            "Join the attack keeps its line break"
        );
    }

    // A PES 2014 save's folder: `select` reads the archive's entry on its own
    // thread and the resolved texts land.
    #[test]
    fn a_pes14_folder_s_archive_reads_the_tooltips() {
        let ctx = egui::Context::default();
        let folder = folder_with_all_str("tips_read");
        let mut tips = GameTips::new();
        tips.select(&ctx, &folder, PesVersion::Pes14);
        let tips = settled_tips(&mut tips).expect("the folder's texts");
        assert_eq!(
            tips.strategies[1].as_deref(),
            Some(
                "Play a tight backline that can win back possession and play long balls to launch counter attacks."
            ),
            "Long Ball's text"
        );
        assert_eq!(
            tips.offside_trap.as_deref(),
            Some(
                "Choose whether or not players will automatically attempt to play offside traps during matches."
            ),
            "Offside trap's block"
        );
    }

    // No folder, a folder holding no English archive, and a save that is not
    // PES 2014's: the read never runs or fails quietly, and `tips` stays None —
    // the screen's tooltips are absent, nothing blocks and nothing panics.
    #[test]
    fn no_folder_or_an_unreadable_one_leaves_no_tooltips() {
        let ctx = egui::Context::default();
        let mut tips = GameTips::new();
        tips.select(&ctx, Path::new(""), PesVersion::Pes14);
        assert!(settled_tips(&mut tips).is_none(), "no folder");

        let folder = scratch("tips_no_archive").join("PES");
        std::fs::create_dir_all(&folder).unwrap();
        tips.select(&ctx, &folder, PesVersion::Pes14);
        assert!(
            settled_tips(&mut tips).is_none(),
            "no dt21_eng.cpk in the folder"
        );

        let folder = folder_with_all_str("tips_pes16");
        tips.select(&ctx, &folder, PesVersion::Pes16);
        assert!(
            settled_tips(&mut tips).is_none(),
            "a PES 2016 save reads nothing"
        );
    }
}
