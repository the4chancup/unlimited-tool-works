//! The window's grid check: `check`'s own validation, run on a thread of its own, its outcome
//! turned there into the rows the grid paints, one per export. Each finding goes to the cell
//! of what it is about: a player's, the portraits', a kit's, or the team label's.

use std::collections::{BTreeMap, BTreeSet};
use std::thread::JoinHandle;

use aesthetics_export::{
    ExportKind, FolderDraft, ParsedAestheticsExport, PlayerSlot, RefSlot, ResolvedAestheticsExport,
    ValidatedRoster, folder_slot, split_folder_name,
};
use export_grid::{Cell, CellState, IdCell, Line, Outline, Position, Row, RowBody, Tone, Width};
use kit_config::KitSlot;
use studio_core::{Message, Scope, Severity, ToolContext};
use teams_list::TeamsList;
use vtree::{ScopePath, fold_name};

use crate::check::validate;
use crate::cli;
use crate::reader::{self, SourceKind};
use crate::validation::CheckedSource;

/// What a check's thread hands back: the rows, or the text of the `error:` line that
/// refused it.
type Outcome = Result<Vec<Row>, String>;

/// The grid's check: the rows of the last check that ended, and the check in flight.
#[derive(Default)]
pub(crate) struct LiveCheck {
    rows: Vec<Row>,
    running: Option<JoinHandle<Outcome>>,
    /// A check was asked for while one ran: it starts once that one ends, since the
    /// running one may predate what made the caller ask.
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
    /// its refusal's text is returned for the log while the grid keeps its rows. Then starts
    /// the check asked for meanwhile. Returns at once while the check runs.
    pub(crate) fn poll(&mut self, ctx: &ToolContext) -> Option<String> {
        let running = self.running.take_if(|running| running.is_finished())?;
        let refusal = match running.join() {
            Ok(Ok(rows)) => {
                self.rows = rows;
                None
            }
            Ok(Err(error)) => Some(error),
            Err(_) => Some("the grid's check stopped because it crashed".to_owned()),
        };
        if std::mem::take(&mut self.again) {
            self.start(ctx);
        }
        refusal
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
}

/// The check a thread runs: `check` with no arguments, its preflight included, each export's
/// outcome made a row.
fn check_rows(ctx: &ToolContext) -> Outcome {
    let inputs = cli::check_inputs(ctx).map_err(|error| error.to_string())?;
    let discovery = reader::discover(&inputs.exports_root, &inputs.exports)
        .map_err(|error| format!("{error:#}"))?;
    let pass = validate(&inputs, discovery.sources).map_err(|error| format!("{error:#}"))?;
    Ok(pass
        .sources
        .into_iter()
        .map(|checked| export_row(checked, &inputs.teams_list))
        .collect())
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
/// its label alone.
fn export_row(checked: CheckedSource, teams_list: &TeamsList) -> Row {
    let source = &checked.source;
    let referee = source.is_referees();
    let shallow = match source.kind {
        SourceKind::Folder => false,
        SourceKind::Zip | SourceKind::SevenZ => true,
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
    let layout = match (&checked.resolved, &checked.parsed) {
        _ if message.is_some() => None,
        (Some(resolved), Some(parsed)) => Some(with_dropped(
            resolved_layout(resolved),
            draft_layout(parsed),
        )),
        (Some(resolved), None) => Some(resolved_layout(resolved)),
        (None, Some(parsed)) => Some(draft_layout(parsed)),
        (None, None) => None,
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
            let lines = lines(&layout, checked.messages, shallow, blocked, &mut label);
            RowBody::Lines(lines)
        }
        (None, None) => {
            label = checked.messages;
            RowBody::Lines(Vec::new())
        }
    };
    Row {
        label: cell(name, label, shallow, false),
        id: Some(id),
        body,
    }
}

/// A cell showing `text` for `messages`: checked (partial-ok when `shallow`, from an archive),
/// ringed by the worst Warning or Error, an Info changing nothing; blocked, with no ring, when
/// its export will not compile.
fn cell(text: String, messages: Vec<Message>, shallow: bool, blocked: bool) -> Cell {
    let worst = messages.iter().map(|message| message.severity).max();
    let (state, outline) = match (blocked, shallow, worst) {
        (true, _, _) => (CellState::Blocked, None),
        (false, true, worst) => (CellState::PartialOk, outline(worst)),
        (false, false, worst) => (CellState::FullOk, outline(worst)),
    };
    Cell {
        text,
        state,
        outline,
        messages,
        width: Width::Square,
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
    /// Whether the export is the referees': its slots run to 35, on two lines.
    referee: bool,
}

/// A validated export's layout: its roster's folders, its kits, its players' portraits.
fn resolved_layout(resolved: &ResolvedAestheticsExport) -> Layout {
    let export = &resolved.export;
    let mut slots: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    let referee = matches!(export.roster, ValidatedRoster::Referees(_));
    let portraits = match &export.roster {
        ValidatedRoster::Team(roster) => {
            for (slot, index) in roster {
                slots.entry(index.0).or_default().push(slot.get());
            }
            let has_portrait = |(slot, index): &(&PlayerSlot, &aesthetics_export::PlayerIndex)| {
                export.portraits.contains_key(*slot) || export.players[index.0].portrait.is_some()
            };
            Some(roster.iter().filter(has_portrait).count())
        }
        ValidatedRoster::Referees(roster) => {
            for (slot, index) in roster {
                slots.entry(index.0).or_default().push(slot.get());
            }
            None
        }
    };
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
        portraits,
        referee,
    }
}

/// `kept`, a validated export's layout, with the folders a finding dropped from it added
/// from `listed`, its listing's layout: each at the slots its listing gives it that no kept
/// folder holds. A dropped folder keeps its cell, so its finding colours its own player or
/// kit rather than the label. The portraits count is `listed`'s: a dropped folder's
/// portrait is still in the export, and the portraits cell counts the players that have
/// one, as it does for an export dropped whole (`draft_layout`).
fn with_dropped(mut kept: Layout, listed: Layout) -> Layout {
    kept.portraits = listed.portraits;
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
/// its name claims; each kit folder at the slot its name claims. Names are read as validation
/// reads them.
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
    let portraits = (!referee).then(|| {
        slots
            .iter()
            .flat_map(|(index, slots)| {
                slots
                    .iter()
                    .map(move |slot| (&draft.players[*index], *slot))
            })
            .filter(|(folder, slot)| has_draft_portrait(parsed, folder, *slot))
            .count()
    });
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
        referee,
    }
}

/// Whether the player at `slot`, in `folder`, has a portrait in the listing: a `portrait.*`
/// file directly in his folder, or a `Portraits/` file named for his slot.
fn has_draft_portrait(parsed: &ParsedAestheticsExport, folder: &FolderDraft, slot: u8) -> bool {
    let in_folder = folder.files.iter().any(|file| {
        file.path.parent().as_ref() == Some(&folder.path)
            && stem(file.path.name()).eq_ignore_ascii_case("portrait")
    });
    let in_portraits = parsed
        .draft
        .portraits
        .iter()
        .any(|file| folder_slot(stem(file.path.name())).map(PlayerSlot::get) == Some(slot));
    in_folder || in_portraits
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

/// The lines of an export laid out as `layout`, each cell holding the `messages` that land in
/// it, the rest going to `label`: one line of 23 player positions, the portraits and the
/// kits, and on a referee row a second line under it for the slots past 23.
fn lines(
    layout: &Layout,
    messages: Vec<Message>,
    shallow: bool,
    blocked: bool,
    label: &mut Vec<Message>,
) -> Vec<Line> {
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
                    Position::Cell(cell(format!("{slot:02}"), messages, shallow, blocked))
                }
                None => Position::Empty,
            })
            .collect()
    };
    let first_players = player_line(1);
    let portraits = match layout.portraits {
        Some(with_portrait) => Position::Cell(Cell {
            width: Width::Wide,
            ..cell(
                format!("{with_portrait}/{player_count}"),
                portraits,
                shallow,
                blocked,
            )
        }),
        None => Position::Empty,
    };
    let kits = KIT_COLUMNS
        .iter()
        .map(|(slot, text)| match kits.remove(slot) {
            Some(messages) => Position::Cell(cell((*text).to_owned(), messages, shallow, blocked)),
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
            Ok(rows)
        });
        LiveCheck {
            rows: Vec::new(),
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
