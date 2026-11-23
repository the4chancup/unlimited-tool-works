//! The window's grid check: `check`'s own validation, run on a thread of its own, its outcome
//! turned there into the rows the grid paints, one per export. Each finding goes to the cell
//! of what it is about: a player's, the portraits', a kit's, or the team label's.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

use aesthetics_export::{
    ExportKind, FileDescriptor, FileKind, FolderDraft, ParsedAestheticsExport, PlayerSlot, RefSlot,
    ResolvedAestheticsExport, ValidatedRoster, folder_slot, is_referee_marker, kit_textures,
    logo_parts, split_folder_name,
};
use egui::Color32;
use export_grid::{
    Cell, CellState, IdCell, KitIcon, Line, Open, Outline, PictureFile, Position, Row, RowBody,
    Tone, TopBox, Width,
};
use kit_config::KitSlot;
use studio_core::{Message, Scope, Severity, ToolContext, message_line};
use teams_list::TeamsList;
use vtree::{ScopePath, fold_name};

use crate::check::validate;
use crate::cli;
use crate::compile;
use crate::reader::{self, ExportSource, SourceKind};
use crate::validation::{CheckedSource, KitMenu};

/// What a check's thread hands back: the rows and the DpFileList findings, or the text of
/// the `error:` line that refused it.
type Outcome = Result<(Vec<Row>, Vec<Message>), String>;

/// What `LiveCheck::poll` found.
#[derive(Debug, PartialEq)]
pub(crate) enum Polled {
    /// No check ended since the last poll.
    Nothing,
    /// A check ended: its rows replaced the grid's, and its `DpFileList.bin` findings the
    /// button's state.
    Rows,
    /// A check ended refused: the text of its `error:` line, for the log.
    Refused(String),
}

/// The grid's check: the rows of the last check that ended and its DpFileList findings,
/// and the check in flight.
#[derive(Default)]
pub(crate) struct LiveCheck {
    rows: Vec<Row>,
    /// The findings about the game's `DpFileList.bin` `rows`' check made.
    list: Vec<Message>,
    running: Option<JoinHandle<Outcome>>,
    /// A check was asked for while one ran: it starts once that one ends, and the ended
    /// one's outcome is dropped, since it may predate what made the caller ask.
    again: bool,
    started: bool,
}

impl LiveCheck {
    /// Starts a check on a thread of its own, or, while one runs, asks for another once it
    /// ends.
    pub(crate) fn start(&mut self, ctx: &ToolContext) {
        self.started = true;
        if self.running.is_some() {
            self.again = true;
            return;
        }
        let ctx = ctx.clone();
        self.running = Some(std::thread::spawn(move || check_rows(&ctx)));
    }

    /// Takes the check's outcome once its thread has ended: its rows replace the grid's, or
    /// the grid keeps its rows and the refusal is returned for the log. The outcome is
    /// dropped instead when another check was asked for while it ran — it may predate what
    /// made the caller ask — and the asked-for check starts. Returns at once while the
    /// check runs.
    pub(crate) fn poll(&mut self, ctx: &ToolContext) -> Polled {
        let Some(running) = self.running.take_if(|running| running.is_finished()) else {
            return Polled::Nothing;
        };
        let outcome = running.join();
        if std::mem::take(&mut self.again) {
            self.start(ctx);
            return Polled::Nothing;
        }
        match outcome {
            Ok(Ok((rows, list))) => {
                // Replaced together: a list older than the rows would gate Compile on a
                // `DpFileList.bin` that may have changed since.
                self.rows = rows;
                self.list = list;
                Polled::Rows
            }
            Ok(Err(error)) => Polled::Refused(error),
            Err(_) => Polled::Refused("the grid's check stopped because it crashed".to_owned()),
        }
    }

    /// Whether a check is in flight.
    pub(crate) fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// Whether a check was ever started.
    pub(crate) fn has_started(&self) -> bool {
        self.started
    }

    /// The rows of the last check that ended, in discovery order.
    pub(crate) fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The last check's findings about the game's `DpFileList.bin` (`compile::list_findings`
    /// as a normal compile's preflight makes them); a refused check keeps the last ones.
    pub(crate) fn list_findings(&self) -> &[Message] {
        &self.list
    }
}

/// The check a thread runs: `check` with no arguments, its preflight included, each export's
/// outcome made a row, and the findings about the game's `DpFileList.bin` a normal compile's
/// preflight would report.
fn check_rows(ctx: &ToolContext) -> Outcome {
    let inputs = cli::check_inputs(ctx).map_err(|error| error.to_string())?;
    let discovery = reader::discover(&inputs.exports_root, &inputs.exports)
        .map_err(|error| format!("{error:#}"))?;
    let referees = discovery.sources.iter().any(ExportSource::is_referees);
    let pass = validate(&inputs, discovery.sources).map_err(|error| format!("{error:#}"))?;
    let rows = pass
        .sources
        .into_iter()
        .map(|checked| export_row(checked, &inputs.teams_list))
        .collect();
    let list = compile::list_findings(&inputs, referees, ctx.paths().data_dir.as_deref());
    Ok((rows, list))
}

/// The message a row shows in place of its cells when `message` is one of the two that leave
/// it none, and its colour: an export whose name says neither `Full` nor `Midcup`, and one in
/// the old layout, which only the Export upgrader reads.
fn row_message(message: &Message) -> Option<(Tone, &'static str)> {
    match message.code.code.as_ref() {
        "export_tag_missing" => Some((Tone::Warning, "Full or Midcup?")),
        "export_layout_old" => Some((Tone::Error, "Old export format")),
        _ => None,
    }
}

/// The player columns of a line: a referee row's slots past them take a second line.
const PLAYER_COLUMNS: u8 = 23;

/// The kit columns, left to right, and the text of each one's cell.
const KIT_COLUMNS: [(KitSlot, &str); 10] = [
    (KitSlot::G1, "GK"),
    (KitSlot::P1, "K1"),
    (KitSlot::P2, "K2"),
    (KitSlot::P3, "K3"),
    (KitSlot::P4, "K4"),
    (KitSlot::P5, "K5"),
    (KitSlot::P6, "K6"),
    (KitSlot::P7, "K7"),
    (KitSlot::P8, "K8"),
    (KitSlot::P9, "K9"),
];

/// `checked`'s row, its team's ID read from `teams_list`. Its label is the export's `/xx/`
/// team name, or its folder or archive name when that starts with no team name. An export
/// set aside before its files were listed (disabled, an archive that cannot be read) shows
/// its label alone. The row is keyed by the export's folder or archive name. Its log lines,
/// which the label's Copy messages copies, are its findings as the console prints them when
/// one is a Warning or worse, and none otherwise: every export the check identifies carries
/// an Info, and a clean export has nothing to send its member.
fn export_row(checked: CheckedSource, teams_list: &TeamsList) -> Row {
    let source = &checked.source;
    let referee = source.is_referees();
    let disk = Disk {
        path: &source.path,
        archive: match source.kind {
            SourceKind::Folder => false,
            SourceKind::Zip | SourceKind::SevenZ => true,
        },
        shallow: source.kind == SourceKind::SevenZ,
    };
    let shallow = disk.shallow;
    // An export set aside before its files were listed shows no box: nothing is known of
    // it. Nor does a `.7z`: reading a picture out of the solid archive decompresses it
    // whole, which only its deep check does.
    let boxes = (source.kind != SourceKind::SevenZ && checked.parsed.is_some()).then(|| Boxes {
        export: source.path.clone(),
        logo: logo_file(&checked, referee).map(|file| picture(&source.path, &file)),
    });
    let to_send = checked
        .messages
        .iter()
        .any(|message| message.severity >= Severity::Warning);
    let log_lines = if to_send {
        checked
            .messages
            .iter()
            .map(|message| message_line(Some(&source.file_name), message))
            .collect()
    } else {
        Vec::new()
    };
    let name = source
        .team_name
        .as_ref()
        .map_or_else(|| source.display_name.clone(), ToString::to_string);
    let id = match source
        .team_name
        .as_ref()
        .and_then(|team| teams_list.id_of(team))
    {
        _ if referee => IdCell::Blank,
        Some(id) => IdCell::Known(id.get().to_string()),
        None => IdCell::Missing,
    };
    let message = checked.messages.iter().find_map(row_message);
    // A dropped export has no validated roster: its cells are its listing's folders.
    // Validation gives every export it resolves its listing too, so none is without one.
    let layout = match (&checked.resolved, &checked.parsed) {
        _ if message.is_some() => None,
        (Some(resolved), Some(parsed)) => {
            let mut layout = with_dropped(resolved_layout(resolved), draft_layout(parsed));
            layout.kit_menus = checked.kit_menus;
            Some(layout)
        }
        (None, Some(parsed)) => Some(draft_layout(parsed)),
        (_, None) => None,
    };
    let blocked = checked.resolved.is_none();
    let mut label = Vec::new();
    let body = match (message, layout) {
        (Some((tone, text)), _) => {
            label = checked.messages;
            RowBody::Message {
                text: text.to_owned(),
                tone,
            }
        }
        (None, Some(layout)) => {
            let lines = lines(
                &layout,
                checked.messages,
                &disk,
                boxes.as_ref(),
                blocked,
                &mut label,
            );
            RowBody::Lines(lines)
        }
        (None, None) => {
            label = checked.messages;
            RowBody::Lines(Vec::new())
        }
    };
    Row {
        key: source.file_name.clone(),
        label: Cell {
            place: source.file_name.clone(),
            open: Some(disk.root()),
            top: boxes.as_ref().map_or(TopBox::Empty, Boxes::label),
            ..cell(name, label, shallow, false)
        },
        id: Some(id),
        body,
        log_lines,
    }
}

/// Where an export is on disk, which its cells' folders open from.
struct Disk<'a> {
    /// The export's folder or archive.
    path: &'a Path,
    /// Whether it is an archive, whose folders have no place of their own on disk.
    archive: bool,
    /// Whether its cells draw the partial-ok ("Shallow check done (archive)") state: a
    /// `.7z`'s alone; a `.zip` gets the folder's deep check.
    shallow: bool,
}

impl Disk<'_> {
    /// What opens the export itself: its folder, or the archive selected in the folder
    /// holding it.
    fn root(&self) -> Open {
        if self.archive {
            Open::Select(self.path.to_owned())
        } else {
            Open::Folder(self.path.to_owned())
        }
    }

    /// What opens `folder`, a folder inside the export, or `None` for a cell with none. Any
    /// cell of an archive opens the archive's own folder, the archive selected.
    fn open(&self, folder: Option<&ScopePath>) -> Option<Open> {
        if self.archive {
            return Some(self.root());
        }
        let mut path = self.path.to_owned();
        for segment in folder?.segments() {
            path.push(segment);
        }
        Some(Open::Folder(path))
    }

    /// A cell for `folder`, a folder inside the export (`cell` for the rest): it prints the
    /// folder and opens it.
    fn cell(
        &self,
        text: String,
        messages: Vec<Message>,
        blocked: bool,
        folder: Option<&ScopePath>,
    ) -> Cell {
        Cell {
            place: folder.map(ScopePath::as_str).unwrap_or_default().to_owned(),
            open: self.open(folder),
            ..cell(text, messages, self.shallow, blocked)
        }
    }
}

/// A cell showing `text` for `messages`: checked (partial-ok when `shallow`, a `.7z`),
/// ringed by the worst Warning or Error, an Info changing nothing; blocked, with no ring, when
/// its export will not compile. It has no folder: `Disk::cell` gives one.
fn cell(text: String, messages: Vec<Message>, shallow: bool, blocked: bool) -> Cell {
    let worst = messages.iter().map(|message| message.severity).max();
    let (state, outline) = match (blocked, shallow, worst) {
        (true, _, _) => (CellState::Blocked, None),
        (false, true, worst) => (CellState::PartialOk, outline(worst)),
        (false, false, worst) => (CellState::FullOk, outline(worst)),
    };
    Cell {
        outline,
        messages,
        ..Cell::new(text, state)
    }
}

/// The ring of a cell whose worst finding is `worst`.
fn outline(worst: Option<Severity>) -> Option<Outline> {
    match worst {
        Some(Severity::Error | Severity::Fatal) => Some(Outline::Error),
        Some(Severity::Warning) => Some(Outline::Warning),
        Some(Severity::Info) | None => None,
    }
}

/// Where an export's findings land: the folders behind its cells.
struct Layout {
    /// Each player folder and the slots it backs (two referee slots may share one).
    players: Vec<(ScopePath, Vec<u8>)>,
    /// Each kit's folder.
    kits: Vec<(KitSlot, ScopePath)>,
    /// How many players have a portrait; `None` when the row has no portraits cell.
    portraits: Option<usize>,
    /// The export's `Portraits/` folder, as its listing spells it; `None` without one.
    portraits_folder: Option<ScopePath>,
    /// Whether the export is the referees': its slots run to 35, on two lines.
    referee: bool,
    /// Each player slot's portrait, the one the compile packs: his folder's own, else his
    /// `Portraits/` file.
    portrait_files: BTreeMap<u8, FileDescriptor>,
    /// Each kit's main texture.
    kit_textures: BTreeMap<KitSlot, FileDescriptor>,
    /// Each kept kit's menu colors and menu icon, by slot; filled from the check where the
    /// layout is a validated export's, empty for one drawn from a listing alone.
    kit_menus: BTreeMap<KitSlot, KitMenu>,
}

/// A validated export's layout: its roster's folders, its kits and their pictures. It counts
/// no portraits: the count is its listing's (`draft_layout`), which `with_dropped` takes.
fn resolved_layout(resolved: &ResolvedAestheticsExport) -> Layout {
    let export = &resolved.export;
    let mut slots: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    let referee = matches!(export.roster, ValidatedRoster::Referees(_));
    match &export.roster {
        ValidatedRoster::Team(roster) => {
            for (slot, index) in roster {
                slots.entry(index.0).or_default().push(slot.get());
            }
        }
        ValidatedRoster::Referees(roster) => {
            for (slot, index) in roster {
                slots.entry(index.0).or_default().push(slot.get());
            }
        }
    }
    let mut portrait_files = BTreeMap::new();
    for (index, slots) in &slots {
        for slot in slots {
            let in_portraits = || {
                let slot = PlayerSlot::new(*slot)?;
                export.portraits.get(&slot)
            };
            if let Some(file) = export.players[*index]
                .portrait
                .as_ref()
                .or_else(in_portraits)
            {
                portrait_files.insert(*slot, file.clone());
            }
        }
    }
    let kit_textures = export
        .kits
        .kits
        .iter()
        .filter_map(|(slot, kit)| {
            let main = kit.textures.iter().find(|texture| texture.stem == "kit")?;
            Some((*slot, main.file.clone()))
        })
        .collect();
    Layout {
        players: slots
            .into_iter()
            .map(|(index, slots)| (export.players[index].path.clone(), slots))
            .collect(),
        kits: export
            .kits
            .kits
            .iter()
            .map(|(slot, kit)| (*slot, kit.path.clone()))
            .collect(),
        portraits: None,
        portraits_folder: None,
        referee,
        portrait_files,
        kit_textures,
        kit_menus: BTreeMap::new(),
    }
}

/// `kept`, a validated export's layout, with the folders a finding dropped from it added
/// from `listed`, its listing's layout: each at the slots its listing gives it that no kept
/// folder holds. A dropped folder keeps its cell, so its finding colours its own player or
/// kit rather than the label. The portraits count is `listed`'s: a dropped folder's
/// portrait is still in the export, and the portraits cell counts the players that have
/// one, as it does for an export dropped whole (`draft_layout`). The pictures stay `kept`'s,
/// the files the compile packs.
fn with_dropped(mut kept: Layout, listed: Layout) -> Layout {
    kept.portraits = listed.portraits;
    kept.portraits_folder = listed.portraits_folder;
    let taken: BTreeSet<u8> = kept
        .players
        .iter()
        .flat_map(|(_, slots)| slots.iter().copied())
        .collect();
    for (folder, slots) in listed.players {
        if kept.players.iter().any(|(path, _)| *path == folder) {
            continue;
        }
        let free: Vec<u8> = slots
            .into_iter()
            .filter(|slot| !taken.contains(slot))
            .collect();
        if !free.is_empty() {
            kept.players.push((folder, free));
        }
    }
    for (slot, folder) in listed.kits {
        if !kept.kits.iter().any(|(held, _)| *held == slot) {
            kept.kits.push((slot, folder));
        }
    }
    kept
}

/// An export's layout as its listing gives it, before any finding dropped a part of it: each
/// player folder at the slots its roster file gives it, or, with no roster file, at the slot
/// its name claims; each kit folder at the slot its name claims; their pictures. Names are
/// read as validation reads them.
fn draft_layout(parsed: &ParsedAestheticsExport) -> Layout {
    let draft = &parsed.draft;
    let referee = draft.kind() == ExportKind::Referees;
    let in_range = |slot: u16| {
        u8::try_from(slot).ok().filter(|&slot| {
            if referee {
                RefSlot::new(slot).is_some()
            } else {
                PlayerSlot::new(slot).is_some()
            }
        })
    };
    let mut slots: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    match &parsed.raw_roster {
        Some(roster) => {
            for entry in &roster.entries {
                let (Some(slot), Some(name)) = (entry.slot.and_then(in_range), &entry.folder_name)
                else {
                    continue;
                };
                let named = |folder: &FolderDraft| fold_name(folder.path.name()) == fold_name(name);
                if let Some(index) = draft.players.iter().position(named) {
                    slots.entry(index).or_default().push(slot);
                }
            }
        }
        None => {
            for (index, folder) in draft.players.iter().enumerate() {
                if let Some(slot) = folder_slot(folder.path.name()) {
                    slots.entry(index).or_default().push(slot.get());
                }
            }
        }
    }
    let at_slots: Vec<(&FolderDraft, u8)> = slots
        .iter()
        .flat_map(|(index, slots)| {
            slots
                .iter()
                .map(move |slot| (&draft.players[*index], *slot))
        })
        .collect();
    let portraits = (!referee).then(|| {
        at_slots
            .iter()
            .filter(|(folder, slot)| draft_portrait(parsed, folder, *slot).is_some())
            .count()
    });
    let portrait_files = at_slots
        .iter()
        .filter_map(|(folder, slot)| Some((*slot, draft_portrait(parsed, folder, *slot)?.clone())))
        .collect();
    let kit_textures = draft
        .kits
        .iter()
        .filter_map(|kit| {
            let (head, _) = split_folder_name(kit.path.name());
            let main = kit_textures(kit)
                .find(|file| stem(file.path.name()).eq_ignore_ascii_case("kit"))?;
            Some((KitSlot::parse(head)?, main.clone()))
        })
        .collect();
    Layout {
        players: slots
            .into_iter()
            .map(|(index, slots)| (draft.players[index].path.clone(), slots))
            .collect(),
        kits: draft
            .kits
            .iter()
            .filter_map(|kit| {
                let (head, _) = split_folder_name(kit.path.name());
                Some((KitSlot::parse(head)?, kit.path.clone()))
            })
            .collect(),
        portraits,
        portraits_folder: draft
            .portraits
            .first()
            .and_then(|file| file.path.segments().next())
            .and_then(|folder| ScopePath::new(folder).ok()),
        referee,
        portrait_files,
        kit_textures,
        kit_menus: BTreeMap::new(),
    }
}

/// The file standing for `checked`'s export in its boxes: a referee export's marker when it
/// has one, since its logo reaches no output while the referees wear the marker in game;
/// else the export's main logo. Taken from the validated export when there is one, else
/// from the export's listing; `None` without either.
fn logo_file(checked: &CheckedSource, referee: bool) -> Option<FileDescriptor> {
    if let Some(resolved) = &checked.resolved {
        let export = &resolved.export;
        let main = export.logo.as_ref().map(|logo| &logo.main.file);
        return export.root.referee_marker.as_ref().or(main).cloned();
    }
    let draft = &checked.parsed.as_ref()?.draft;
    let marker = draft.root_files.iter().find(|file| {
        referee && file.kind == FileKind::Texture && is_referee_marker(file.path.name())
    });
    let main = draft.root_files.iter().find(|file| {
        file.kind == FileKind::Texture
            && logo_parts(stem(file.path.name())).is_some_and(|(small, _)| !small)
    });
    marker.or(main).cloned()
}

/// What an export's top boxes show: its files as pictures, and the file standing for the
/// export (`logo_file`). A `.7z` export is never given one: reading a picture out of the
/// solid archive would decompress it whole.
struct Boxes {
    /// The export's folder, or its `.zip` file.
    export: PathBuf,
    /// The logo, or a referee export's marker.
    logo: Option<PictureFile>,
}

/// `file`, a file of the export `export`, as the picture a window reads. `export` is the
/// export's folder, or its `.zip` file; `path` is the file's `source`: an entry name for
/// a `.zip`, and the path on disk for a folder, which differs from the export's own under
/// a folder layer the listing flattened.
fn picture(export: &Path, file: &FileDescriptor) -> PictureFile {
    PictureFile {
        export: export.to_owned(),
        path: file.source.as_str().to_owned(),
    }
}

impl Boxes {
    /// A label's box: the logo.
    fn label(&self) -> TopBox {
        TopBox::Logo(self.logo.clone())
    }

    /// A player's box: his `portrait` and the logo.
    fn player(&self, portrait: Option<&FileDescriptor>) -> TopBox {
        TopBox::Player {
            portrait: portrait.map(|file| picture(&self.export, file)),
            logo: self.logo.clone(),
        }
    }

    /// A kit's box: its main `texture`, or nothing without one.
    fn kit(&self, texture: Option<&FileDescriptor>) -> TopBox {
        texture.map_or(TopBox::Empty, |file| {
            TopBox::Picture(picture(&self.export, file))
        })
    }

    /// The portraits cell's box: `portraits` in slot order, and the logo.
    fn portraits<'a>(&self, portraits: impl Iterator<Item = Option<&'a FileDescriptor>>) -> TopBox {
        TopBox::Portraits {
            portraits: portraits
                .map(|portrait| portrait.map(|file| picture(&self.export, file)))
                .collect(),
            logo: self.logo.clone(),
        }
    }
}

/// The portrait in the listing of the player at `slot`, in `folder`: a `portrait.*` file
/// directly in his folder, else a `Portraits/` file named for his slot.
fn draft_portrait<'a>(
    parsed: &'a ParsedAestheticsExport,
    folder: &'a FolderDraft,
    slot: u8,
) -> Option<&'a FileDescriptor> {
    let in_folder = folder.files.iter().find(|file| {
        file.path.parent().as_ref() == Some(&folder.path)
            && stem(file.path.name()).eq_ignore_ascii_case("portrait")
    });
    let in_portraits = || {
        parsed
            .draft
            .portraits
            .iter()
            .find(|file| folder_slot(stem(file.path.name())).map(PlayerSlot::get) == Some(slot))
    };
    in_folder.or_else(in_portraits)
}

/// A file name without its extension.
fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// Where one finding lands.
enum Target {
    /// The cells of these player slots.
    Players(Vec<u8>),
    /// The portraits' cell.
    Portraits,
    /// This kit's cell.
    Kit(KitSlot),
    /// The team label.
    Label,
}

/// The cell a finding about `scope` lands in: a player folder or a file in one (a portrait
/// included) is every slot that folder backs; a `Portraits/` file the portraits' cell; a kit
/// folder or a file in one that kit's cell; anything else (the export, a shared folder,
/// `Common/`, `Collars/`, a root file, a player folder at no slot) the label.
fn target(scope: &Scope, layout: &Layout) -> Target {
    let path = match scope {
        Scope::Folder { path, .. } | Scope::File { path, .. } => path,
        Scope::Run | Scope::Export { .. } | Scope::RosterEntry { .. } => return Target::Label,
    };
    if let Some((_, slots)) = layout
        .players
        .iter()
        .find(|(folder, _)| within(path, folder))
    {
        return Target::Players(slots.clone());
    }
    let in_portraits = path
        .segments()
        .next()
        .is_some_and(|folder| folder.eq_ignore_ascii_case("Portraits"));
    if in_portraits && layout.portraits.is_some() {
        return Target::Portraits;
    }
    match layout.kits.iter().find(|(_, folder)| within(path, folder)) {
        Some((slot, _)) => Target::Kit(*slot),
        None => Target::Label,
    }
}

/// Whether `path` is `folder` or inside it. Both are spelled as the export's listing spells
/// them, so their text is compared as it is.
fn within(path: &ScopePath, folder: &ScopePath) -> bool {
    path.as_str()
        .strip_prefix(folder.as_str())
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The lines of an export laid out as `layout`, on `disk`, each cell holding the `messages`
/// that land in it, the rest going to `label`, and its pictures from `boxes` (`None`: no
/// pictures): one line of 23 player positions, the portraits and the kits, and on a referee
/// row a second line under it for the slots past 23.
fn lines(
    layout: &Layout,
    messages: Vec<Message>,
    disk: &Disk,
    boxes: Option<&Boxes>,
    blocked: bool,
    label: &mut Vec<Message>,
) -> Vec<Line> {
    let shallow = disk.shallow;
    let folders: BTreeMap<u8, &ScopePath> = layout
        .players
        .iter()
        .flat_map(|(folder, slots)| slots.iter().map(move |slot| (*slot, folder)))
        .collect();
    let mut players: BTreeMap<u8, Vec<Message>> = layout
        .players
        .iter()
        .flat_map(|(_, slots)| slots)
        .map(|slot| (*slot, Vec::new()))
        .collect();
    let mut kits: BTreeMap<KitSlot, Vec<Message>> = layout
        .kits
        .iter()
        .map(|(slot, _)| (*slot, Vec::new()))
        .collect();
    let mut portraits = Vec::new();
    for message in messages {
        match target(&message.scope, layout) {
            Target::Players(slots) => {
                for slot in slots {
                    players.entry(slot).or_default().push(message.clone());
                }
            }
            Target::Portraits => portraits.push(message),
            Target::Kit(slot) => kits.entry(slot).or_default().push(message),
            Target::Label => label.push(message),
        }
    }
    let player_count = players.len();
    let mut player_line = |first: u8| -> Vec<Position> {
        (first..first + PLAYER_COLUMNS)
            .map(|slot| match players.remove(&slot) {
                Some(messages) => {
                    let folder = folders.get(&slot).copied();
                    let portrait = layout.portrait_files.get(&slot);
                    Position::Cell(Box::new(Cell {
                        top: boxes.map_or(TopBox::Empty, |boxes| boxes.player(portrait)),
                        ..disk.cell(format!("{slot:02}"), messages, blocked, folder)
                    }))
                }
                None => Position::Empty,
            })
            .collect()
    };
    let first_players = player_line(1);
    let portraits = match layout.portraits {
        Some(with_portrait) => {
            // With no `Portraits/`, each portrait is in its player's folder, and a finding
            // about one is on that player's cell: this cell has no folder.
            let folder = layout.portraits_folder.as_ref();
            Position::Cell(Box::new(Cell {
                width: Width::Wide,
                place: folder.map_or_else(
                    || "Portraits in the player folders".to_owned(),
                    |folder| folder.as_str().to_owned(),
                ),
                open: disk.open(folder),
                top: boxes.map_or(TopBox::Empty, |boxes| {
                    let slots = 1..=PLAYER_COLUMNS;
                    boxes.portraits(slots.map(|slot| layout.portrait_files.get(&slot)))
                }),
                ..cell(
                    format!("{with_portrait}/{player_count}"),
                    portraits,
                    shallow,
                    blocked,
                )
            }))
        }
        None => Position::Empty,
    };
    let kit_folders: BTreeMap<KitSlot, &ScopePath> = layout
        .kits
        .iter()
        .map(|(slot, folder)| (*slot, folder))
        .collect();
    let kits = KIT_COLUMNS
        .iter()
        .map(|(slot, text)| match kits.remove(slot) {
            Some(messages) => {
                let folder = kit_folders.get(slot).copied();
                let texture = layout.kit_textures.get(slot);
                Position::Cell(Box::new(Cell {
                    kit_icon: layout.kit_menus.get(slot).map(|menu| KitIcon {
                        number: menu.icon,
                        colors: menu.colors.map(|[r, g, b]| Color32::from_rgb(r, g, b)),
                    }),
                    top: boxes.map_or(TopBox::Empty, |boxes| boxes.kit(texture)),
                    ..disk.cell((*text).to_owned(), messages, blocked, folder)
                }))
            }
            None => Position::Empty,
        })
        .collect();
    let mut lines = vec![Line {
        name: None,
        groups: vec![first_players, vec![portraits], kits],
    }];
    if layout.referee {
        lines.push(Line {
            name: None,
            groups: vec![
                player_line(PLAYER_COLUMNS + 1),
                vec![Position::Empty],
                vec![Position::Empty; KIT_COLUMNS.len()],
            ],
        });
    }
    lines
}

#[cfg(test)]
impl LiveCheck {
    /// A check in flight until `until` delivers, which then ends with `rows`: a check that
    /// stays running for as long as a test needs.
    pub(crate) fn in_flight(until: crossbeam_channel::Receiver<()>, rows: Vec<Row>) -> LiveCheck {
        let running = std::thread::spawn(move || {
            until
                .recv()
                .map_err(|_| "the test dropped its check".to_owned())?;
            Ok((rows, Vec::new()))
        });
        LiveCheck {
            rows: Vec::new(),
            list: Vec::new(),
            running: Some(running),
            again: false,
            started: true,
        }
    }

    /// Whether the check in flight has ended, its outcome not taken yet.
    pub(crate) fn has_ended(&self) -> bool {
        self.running.as_ref().is_some_and(JoinHandle::is_finished)
    }
}

#[cfg(test)]
mod tests {
    use crossbeam_channel::unbounded;
    use export_grid::{Cell, CellState, RowBody};

    use super::*;
    use crate::testing::{poll_until_done, sandbox, tool_context};

    /// A row the ended check's outcome holds.
    fn stale_row() -> Row {
        Row {
            key: "co Midcup Map".to_owned(),
            label: Cell::new("/co/", CellState::FullOk),
            id: None,
            body: RowBody::Lines(Vec::new()),
            log_lines: Vec::new(),
        }
    }

    // A check asked for while one ran starts once it ends, and the ended one's outcome is
    // dropped: it may predate what made the caller ask (an upgrade run, whose own drained
    // findings set the button's state from the newer list).
    #[test]
    fn a_check_ended_while_another_was_asked_for_drops_its_outcome() {
        let temp = sandbox("live_check_stale");
        let ctx = tool_context(temp.path(), "");
        let (release, until) = unbounded();
        let mut check = LiveCheck::in_flight(until, vec![stale_row()]);
        check.start(&ctx);
        release.send(()).unwrap();
        poll_until_done(|| !check.has_ended());

        assert_eq!(check.poll(&ctx), Polled::Nothing);
        assert!(
            check.rows().is_empty(),
            "the ended check's rows are dropped"
        );
        assert!(check.is_running(), "the asked-for check is running");
    }
}
