//! The team file panel (the draft's `interchangePanel`): an `egui::Window`
//! whose first row exports the team selected in the editor as a Team TOML
//! or a Texport, through a Save dialog that refuses the open save itself,
//! and whose rest imports a team file onto that team, the file
//! deciding the format: a Team TOML, a squad (`.4ccs`) or tactics (`.4cct`)
//! file, or a Texport. The member picks which of the sections the file
//! holds load (gameplay, aesthetics, tactics), reads what the import will
//! not write as written (the catalog's text once per note code, the notes
//! grouped by key in words with their players) and, for a
//! squad file, who gets which record. The staged import is the file applied
//! to copies of the target team and its players, recomputed when the save
//! changes (an edit, an undo, another save opened), the target or the
//! choices, so Apply writes what the panel shows, as one undoable edit; an
//! import that would change nothing disables Apply and says so. A
//! file that cannot be read is refused alone, in the command line's words;
//! a refusal of the chosen sections' keys is drawn under the choices, which
//! stay drawn, since unticking that section is how the member imports the
//! rest. Under the aesthetics lock the aesthetics section is never written.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use pes_savefile::EditFile;
use pes_savefile::file::fresh_salt;
use pes_savefile::interchange::legacy::{read_squad, read_tactics};
use pes_savefile::interchange::team_toml::labels::{card_name, stat_name};
use pes_savefile::interchange::team_toml::{ImportNote, TeamToml, TeamTomlError};
use pes_savefile::interchange::texport::Texport;
use pes_savefile::model::names::display_name;
use pes_savefile::model::{PlayerEntry, TeamEntry};
use pes_savefile::schema::texport::texport_layout;
use pes_version::PesVersion;
use studio_core::widgets::{checkbox, collapsing_header};
use studio_core::{Notice, NoticeKind};
use team_widgets::names::append_name;

use crate::messages::{import_note, when_and_effect};
use crate::session::Session;
use crate::session::import_sections::{Sections, keep_sections};
use crate::view::appearance::OWNERSHIP_TIP;
use crate::view::comparator::{self, OpenSaveDialog};
use crate::view::{Panel, Refusal, SaveView, selected_team_id, surrender_focus};

/// The panel's Pick dialog: `rfd` on the team file extensions, and every
/// file, since a PES 2015 to 2017 Texport (`TEXPORT00000000`) has none.
#[cfg(not(test))]
pub(super) fn default_interchange_dialog() -> OpenSaveDialog {
    Box::new(|| {
        rfd::FileDialog::new()
            .add_filter("Team files", &["toml", "4ccs", "4cct", "ted"])
            .add_filter("All files", &["*"])
            .pick_file()
    })
}

/// `default_interchange_dialog` under `cargo test`: a test reaching a real
/// file dialog panics; tests stub `view.interchange_dialog`.
#[cfg(test)]
pub(super) fn default_interchange_dialog() -> OpenSaveDialog {
    Box::new(|| panic!("a test reached a real file dialog: stub interchange_dialog"))
}

/// The export row's Save dialog: the format written and the suggested
/// path (the open save's folder and the file's name) to the picked path.
pub(super) type ExportDialog = Box<dyn FnMut(Export, &Path) -> Option<PathBuf>>;

/// The `rfd` Save dialog for the export row, filtered on the format's
/// extension.
#[cfg(not(test))]
pub(super) fn default_export_dialog() -> ExportDialog {
    Box::new(|export, suggested| {
        let mut dialog = rfd::FileDialog::new().add_filter(export.name(), &[export.extension()]);
        if let Some(folder) = suggested.parent() {
            dialog = dialog.set_directory(folder);
        }
        if let Some(name) = suggested.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        dialog.save_file()
    })
}

/// `default_export_dialog` under `cargo test`: a test reaching a real file
/// dialog panics; tests stub `view.team_export_dialog`.
#[cfg(test)]
pub(super) fn default_export_dialog() -> ExportDialog {
    Box::new(|_, _| panic!("a test reached a real file dialog: stub team_export_dialog"))
}

/// The file the export row writes of the selected team.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Export {
    /// The command line's `export-toml` text.
    TeamToml,
    /// A Texport of the save's version, which PES 2018 to 2021 import
    /// from Edit mode.
    Texport,
}

impl Export {
    /// The format's name: the Save dialog's filter, and the refusal of the
    /// open save.
    pub(super) fn name(self) -> &'static str {
        match self {
            Export::TeamToml => "Team TOML",
            Export::Texport => "Texport",
        }
    }

    /// The extension the Save dialog filters on.
    pub(super) fn extension(self) -> &'static str {
        match self {
            Export::TeamToml => "toml",
            Export::Texport => "ted",
        }
    }

    /// The suggested file name for `team`: `<id>.toml`, or
    /// `<short name>.ted` (the id when the short name is empty).
    fn file_name(self, team: &TeamEntry) -> String {
        match self {
            Export::TeamToml => format!("{}.toml", team.id),
            Export::Texport if team.short_name.is_empty() => format!("{}.ted", team.id),
            Export::Texport => format!("{}.ted", team.short_name),
        }
    }
}

/// Every section chosen: the choices as the panel starts.
const ALL_SECTIONS: Sections = Sections {
    gameplay: true,
    aesthetics: true,
    tactics: true,
};

/// The panel's state on `SaveView` (`view.interchange`): the picked file,
/// the section choices, the staged import and the save state Apply left.
pub(super) struct Interchange {
    /// The last picked file, `None` before a pick.
    picked: Option<Picked>,
    /// The member's section choices, all on at first and kept for the
    /// panel's life (never a setting). The aesthetics lock masks the
    /// aesthetics choice without changing it.
    sections: Sections,
    /// The picked file staged onto the target and the key it was staged
    /// for; `Err` is a squad file that does not read against the target.
    staged: Option<(StageKey, Result<Staged, String>)>,
    /// `Session::revision` right after Apply's edit: the Applied line
    /// stands while the save is still in it.
    applied: Option<u64>,
}

/// The format the picked file is read as, from its name.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Format {
    /// `.toml`: a Team TOML.
    TeamToml,
    /// `.4ccs`: a squad file.
    Squad,
    /// `.4cct`: a tactics file.
    Tactics,
    /// Any other name: a Texport (`.ted`, or PES 2015 to 2017's
    /// extension-less `TEXPORT00000000`).
    Texport,
}

impl Format {
    /// The format `path`'s extension names.
    fn of(path: &Path) -> Format {
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
        match extension.as_deref() {
            Some("toml") => Format::TeamToml,
            Some("4ccs") => Format::Squad,
            Some("4cct") => Format::Tactics,
            _ => Format::Texport,
        }
    }

    /// The intro line's opening words.
    fn label(self) -> &'static str {
        match self {
            Format::TeamToml => "A Team TOML",
            Format::Squad => "A legacy squad file (.4ccs)",
            Format::Tactics => "A legacy tactics file (.4cct)",
            Format::Texport => "A Texport",
        }
    }
}

/// The picked file: its path, its format and what reading it gave.
struct Picked {
    /// The file the member picked.
    path: PathBuf,
    /// The format its name names.
    format: Format,
    /// The file read, or the refusal (`<file name>: <error>`).
    read: Result<Read, String>,
}

/// A picked file as read.
enum Read {
    /// A Team TOML, a tactics file or a Texport: its document, read once at
    /// the pick.
    Document(Box<TeamToml>),
    /// A squad file's bytes: its records land on the target's rostered
    /// players, so it is read again whenever the target or the save changes.
    Squad(Vec<u8>),
}

/// What a staged import was computed for: any change of it stages again.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StageKey {
    /// `Session::revision`, which never repeats, across opens included.
    revision: u64,
    /// The target team's id.
    team: u32,
    /// The member's choices.
    sections: Sections,
    /// Whether the aesthetics lock is on.
    locked: bool,
}

/// The picked file against the target.
struct Staged {
    /// The document's `pes_version` (`None`: it names none).
    version: Option<PesVersion>,
    /// The sections the document holds keys of.
    held: Sections,
    /// A squad file's records against the players they land on, in roster
    /// order; empty for the other formats.
    pairing: Vec<Pairing>,
    /// The chosen sections applied, or the lib's refusal of them.
    import: Result<Import, String>,
}

/// One squad record and the rostered player it lands on.
struct Pairing {
    /// The roster slot, 1-based.
    slot: u8,
    /// The record's name, colour codes dropped.
    record: String,
    /// The rostered player's shirt number.
    number: u16,
    /// The rostered player's name as stored, colour codes included.
    name: String,
}

/// The chosen sections applied to copies of the target: what Apply writes
/// and what it does not.
struct Import {
    /// The target team as the import leaves it.
    team: TeamEntry,
    /// The target's rostered players as the import leaves them.
    players: Vec<PlayerEntry>,
    /// Whether the import leaves the team or one of its players other than
    /// the save holds them: compared once, when staged, not every frame.
    changes: bool,
    /// The notes grouped by code and path, in first-seen order.
    notes: Vec<NoteGroup>,
}

/// The notes sharing a code and a path, every player's slot counted as one
/// path.
struct NoteGroup {
    /// `<code> <path>`, a player's slot written `*`.
    key: String,
    /// The note code, whose catalog text is drawn once over its groups.
    code: String,
    /// The catalog's `when` and `effect` for the code, as one line.
    text: String,
    /// The key in words (`Manager ID`, `Tight Possession`), or `Empty
    /// roster place`.
    subject: String,
    /// One per note, in the order the import gave them.
    notes: Vec<NoteRow>,
}

/// One note of a group, as the group's list draws it.
struct NoteRow {
    /// The roster place the note is about (1-based, as the file's
    /// `players.NN` spells it); `None` for a team or tactics key.
    place: Option<u8>,
    /// The target team's player at that place as stored, colour codes
    /// included; `None` when the place holds no player.
    name: Option<String>,
    /// What the note carries: a playing style's name, a lowered value's
    /// `19 → 0`, the cropped text; empty when it carries nothing.
    detail: String,
}

/// What a click in the window asks for, done after the window is drawn.
enum Action {
    /// `Team TOML…` or `Texport…`.
    Export(Export),
    /// `Pick…`.
    Pick,
    /// `Apply`.
    Apply,
}

impl Interchange {
    /// No file picked, every section chosen.
    pub(super) fn new() -> Interchange {
        Interchange {
            picked: None,
            sections: ALL_SECTIONS,
            staged: None,
            applied: None,
        }
    }

    /// Reads the file at `path` as its name's format, dropping the last
    /// file's staged import and Applied line.
    fn pick(&mut self, path: PathBuf) {
        let format = Format::of(&path);
        let read = read(&path, format).map_err(|error| format!("{}: {error}", file_name(&path)));
        self.picked = Some(Picked { path, format, read });
        self.staged = None;
        self.applied = None;
    }

    /// Stages the picked file onto `team` again when `key` differs from
    /// the one it was staged for: once per change, not every frame.
    fn refresh(&mut self, file: &EditFile, team: &TeamEntry, key: StageKey) {
        let Some(Picked {
            path,
            read: Ok(read),
            ..
        }) = &self.picked
        else {
            return;
        };
        if self
            .staged
            .as_ref()
            .is_some_and(|(staged_for, _)| *staged_for == key)
        {
            return;
        }
        let (document, pairing) = match read {
            Read::Document(document) => (Cow::Borrowed(&**document), Vec::new()),
            Read::Squad(bytes) => match read_squad(bytes, team, file.players()) {
                Ok(document) => {
                    let pairing = pairing(file, team, &document);
                    (Cow::Owned(document), pairing)
                }
                Err(error) => {
                    self.staged = Some((key, Err(format!("{}: {error}", file_name(path)))));
                    return;
                }
            },
        };
        let chosen = Sections {
            aesthetics: key.sections.aesthetics && !key.locked,
            ..key.sections
        };
        let staged = Staged {
            version: document.pes_version,
            held: held(&document),
            pairing,
            import: stage(file, team, &document, chosen).map_err(|error| error.to_string()),
        };
        self.staged = Some((key, Ok(staged)));
    }
}

/// The bytes at `path` read as `format`.
fn read(path: &Path, format: Format) -> anyhow::Result<Read> {
    let bytes = std::fs::read(path)?;
    let document = match format {
        Format::TeamToml => TeamToml::parse(std::str::from_utf8(&bytes)?)?,
        Format::Squad => return Ok(Read::Squad(bytes)),
        Format::Tactics => read_tactics(&bytes)?,
        Format::Texport => Texport::from_bytes(&bytes, None)?.to_team_toml()?,
    };
    Ok(Read::Document(Box::new(document)))
}

/// `path`'s file name, the whole path when it has none.
fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// The sections `document` holds. A section is held when keeping it alone
/// leaves another document than keeping none. Read from the document, not
/// from the format: a squad file carries tactics only when it has the
/// tactics block, and a Team TOML written by hand may hold any part.
fn held(document: &TeamToml) -> Sections {
    let kept = |sections: Sections| {
        let mut kept = document.clone();
        keep_sections(&mut kept, sections);
        kept
    };
    let none = kept(Sections {
        gameplay: false,
        aesthetics: false,
        tactics: false,
    });
    let holds = |sections: Sections| kept(sections) != none;
    Sections {
        gameplay: holds(Sections {
            gameplay: true,
            aesthetics: false,
            tactics: false,
        }),
        aesthetics: holds(Sections {
            gameplay: false,
            aesthetics: true,
            tactics: false,
        }),
        tactics: holds(Sections {
            gameplay: false,
            aesthetics: false,
            tactics: true,
        }),
    }
}

/// Each player table of a squad file's `document` against the player
/// `team`'s roster holds at its slot.
fn pairing(file: &EditFile, team: &TeamEntry, document: &TeamToml) -> Vec<Pairing> {
    document
        .players
        .iter()
        .map(|(&slot, section)| {
            let entry = team.roster[usize::from(slot - 1)];
            let player = file
                .player(entry.player_id)
                .expect("a squad record lands on a player of the save");
            Pairing {
                slot,
                record: display_name(
                    section
                        .name
                        .as_deref()
                        .expect("a squad record always carries its name"),
                ),
                number: entry.number,
                name: player.name.clone(),
            }
        })
        .collect()
}

/// `document` reduced to `chosen` and applied onto `team` and its rostered
/// players. The import runs on copies, never on the save: the panel shows
/// its notes and refusal before anything is written, and Apply then writes
/// the copies as one undoable edit.
fn stage(
    file: &EditFile,
    team: &TeamEntry,
    document: &TeamToml,
    chosen: Sections,
) -> Result<Import, TeamTomlError> {
    let mut kept = document.clone();
    keep_sections(&mut kept, chosen);
    let rostered: Vec<PlayerEntry> = file
        .players()
        .iter()
        .filter(|player| {
            player.id != 0 && team.roster.iter().any(|slot| slot.player_id == player.id)
        })
        .cloned()
        .collect();
    let mut imported = team.clone();
    let mut players = rostered.clone();
    let notes = kept.apply(file.version(), &mut imported, &mut players)?;
    let mut groups: Vec<NoteGroup> = Vec::new();
    for note in &notes {
        let message = import_note(note);
        let key = group_key(&message.code.code, note);
        let row = note_row(file, team, note);
        match groups.iter_mut().find(|group| group.key == key) {
            Some(group) => group.notes.push(row),
            None => {
                let (when, effect) = when_and_effect(&message);
                groups.push(NoteGroup {
                    key,
                    code: message.code.code.to_string(),
                    text: format!("{when} {effect}"),
                    subject: note_subject(file.version(), note),
                    notes: vec![row],
                });
            }
        }
    }
    Ok(Import {
        changes: imported != *team || players != rostered,
        team: imported,
        players,
        notes: groups,
    })
}

/// A note's group: its `code` and its path, `players.NN.` written
/// `players.*.` so one key over every player is one group. An empty slot's
/// note has no path and keys by its code alone, so every empty place the
/// file names is one group too.
fn group_key(code: &str, note: &ImportNote) -> String {
    let Some(path) = note_path(note) else {
        return code.to_owned();
    };
    let path = match path
        .strip_prefix("players.")
        .and_then(|rest| rest.split_once('.'))
    {
        Some((_, tail)) => format!("players.*.{tail}"),
        None => path.to_owned(),
    };
    format!("{code} {path}")
}

/// The key path `note` is about; `None` for an empty slot's note, which
/// names a roster place and no key.
fn note_path(note: &ImportNote) -> Option<&str> {
    match note {
        ImportNote::NotInThisVersion { path }
        | ImportNote::NotEncodable { path, .. }
        | ImportNote::Capped { path, .. }
        | ImportNote::TextCropped { path, .. } => Some(path),
        ImportNote::EmptySlot { .. } => None,
    }
}

/// `note`'s row: the place its `players.NN.` path or empty slot names,
/// `team`'s player at that place, and what the note carries.
fn note_row(file: &EditFile, team: &TeamEntry, note: &ImportNote) -> NoteRow {
    let (place, detail) = match note {
        ImportNote::NotInThisVersion { path } => (path_place(path), String::new()),
        ImportNote::NotEncodable { path, label } => (path_place(path), card_name(label)),
        ImportNote::Capped { path, from, to } => {
            (path_place(path), format!("{from} \u{2192} {to}"))
        }
        ImportNote::EmptySlot { slot } => (Some(*slot), String::new()),
        ImportNote::TextCropped { path, cropped } => (path_place(path), display_name(cropped)),
    };
    let name = place
        .and_then(|place| team.roster.get(usize::from(place).checked_sub(1)?))
        .and_then(|slot| file.player(slot.player_id))
        .map(|player| player.name.clone());
    NoteRow {
        place,
        name,
        detail,
    }
}

/// The place a `players.NN.` path names; `None` for a team or tactics key.
fn path_place(path: &str) -> Option<u8> {
    let (place, _) = path.strip_prefix("players.")?.split_once('.')?;
    place.parse().ok()
}

/// `note`'s key in words, as the views name it at the save's `version`: a
/// stat by its name in that game, a skill or COM style by its card name, a
/// motion, an edit flag and a preset's instructions with what they belong
/// to, any other key by its last part (`Manager ID`); an empty slot's note
/// is `Empty roster place`.
fn note_subject(version: PesVersion, note: &ImportNote) -> String {
    let Some(path) = note_path(note) else {
        return "Empty roster place".to_owned();
    };
    let parts: Vec<&str> = path.rsplit('.').collect();
    let last = parts[0];
    match parts.get(1).copied().unwrap_or("") {
        "stats" => stat_name(version, last),
        "skills" | "com_styles" => card_name(last),
        "motion" => format!("{} motion", key_words(last)),
        "edit_flags" => format!("{} edit flag", key_words(last)),
        "instructions" => {
            let side = if last == "attack" {
                "attacking"
            } else {
                "defending"
            };
            let preset = parts.get(2).copied().unwrap_or("");
            format!("{} {side} instructions", key_words(preset))
        }
        _ => key_words(last),
    }
}

/// A key's part in words: underscores as spaces, `id` as ID, `color` as
/// colour, the first letter a capital (`manager_id` reads `Manager ID`).
fn key_words(part: &str) -> String {
    let words = part
        .split('_')
        .map(|word| match word {
            "id" => "ID",
            "color" => "colour",
            word => word,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => words,
    }
}

/// The window for one frame, while it is the open panel and a save is open.
/// The import is staged to the save's state before anything is drawn, so a
/// frame never draws, nor Apply writes, an import an edit made stale.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.panel != Some(Panel::Interchange) {
        return;
    }
    let locked = view.aesthetics_locked();
    let target = selected_team_id(view);
    let SaveView {
        panel,
        session,
        interchange,
        interchange_dialog,
        team_export_dialog,
        refusal,
        notice,
        ..
    } = view;
    let Some(session) = session.as_mut() else {
        return;
    };
    let revision = session.revision();
    let file = session.file();
    let version = file.version();
    let target = target.and_then(|id| file.team(id));
    if let Some(team) = target {
        let key = StageKey {
            revision,
            team: team.id,
            sections: interchange.sections,
            locked,
        };
        interchange.refresh(file, team, key);
    }
    let chosen = interchange.sections;
    let mut open = true;
    let action = comparator::panel_window(ui, "Team file interchange", "save_editor_interchange")
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            contents(ui, interchange, target, version, locked, revision)
        })
        .and_then(|response| response.inner)
        .flatten();
    if !open {
        *panel = None;
    }
    // A choice changed on this frame: the next one stages it.
    if interchange.sections != chosen {
        ui.ctx().request_repaint();
    }
    match action {
        Some(Action::Export(export)) => {
            surrender_focus(ui);
            let team = target.expect("the export row is drawn for a selected team");
            let suggested = session.path().with_file_name(export.file_name(team));
            if let Some(path) = team_export_dialog(export, &suggested) {
                match write_export(session, team.id, export, &path) {
                    Ok(()) => {
                        *notice = Some(Notice {
                            text: format!("Wrote {}", file_name(&path)),
                            kind: NoticeKind::Success,
                            action: None,
                        });
                    }
                    Err(error) => {
                        *refusal = Some(Refusal {
                            title: "Cannot export the team",
                            message: format!("{error:#}"),
                        });
                    }
                }
            }
        }
        Some(Action::Pick) => {
            surrender_focus(ui);
            if let Some(path) = interchange_dialog() {
                interchange.pick(path);
            }
        }
        Some(Action::Apply) => {
            if let Some((
                _,
                Ok(Staged {
                    import: Ok(import), ..
                }),
            )) = &interchange.staged
            {
                session.edit_entries(import.players.clone(), vec![import.team.clone()]);
                interchange.applied = Some(session.revision());
            }
        }
        None => {}
    }
}

/// Writes team `id` of the open save as `export` to `path`. A `path` that is
/// the open save itself is refused before anything is written, as the
/// command line refuses its `--out`: compared canonicalized, so one file
/// under two names still matches, and a path that does not exist yet is no
/// match.
fn write_export(session: &Session, id: u32, export: Export, path: &Path) -> anyhow::Result<()> {
    if let (Ok(destination), Ok(save)) = (
        std::fs::canonicalize(path),
        std::fs::canonicalize(session.path()),
    ) && destination == save
    {
        bail!(
            "{}: that is the save being read, and a {} written over it would lose it",
            path.display(),
            export.name()
        );
    }
    let bytes = match export {
        Export::TeamToml => session.export_team(id)?.into_bytes(),
        Export::Texport => texport_bytes(session, id)?,
    };
    std::fs::write(path, bytes)
        .map_err(|error| anyhow!("{}: cannot write the file ({error})", path.display()))
}

/// Team `id` of the open save as a Texport of the save's version: the team
/// and its rostered players in roster order, from the version's template.
fn texport_bytes(session: &Session, id: u32) -> anyhow::Result<Vec<u8>> {
    let file = session.file();
    let team = file
        .team(id)
        .ok_or_else(|| anyhow!("the save has no team {id}"))?;
    let players = team
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .map(|slot| {
            file.player(slot.player_id)
                .ok_or_else(|| anyhow!("team {id}'s player {} is not in the save", slot.player_id))
        })
        .collect::<anyhow::Result<Vec<&PlayerEntry>>>()?;
    // The salt's first 32 bytes become the file's key. Seeded with the
    // team's and its players' ids as well as the clock, two teams exported
    // in one instant still get different keys, as every file the game
    // writes has its own.
    let ids: Vec<u8> = std::iter::once(id)
        .chain(players.iter().map(|player| player.id))
        .flat_map(u32::to_le_bytes)
        .collect();
    let salt = fresh_salt(&ids);
    let texport = Texport::new(file.version(), team, &players, &salt)?;
    Ok(texport.to_bytes(&salt)?)
}

/// The window's contents: the export row and a separator, the target and
/// the pick row, then once a file is staged its intro line, a squad file's
/// pairing, the section choices, the refusal or the notes, `Apply`, then
/// the Applied line or the line saying the team already holds the file. A
/// file that does not read is its refusal alone.
fn contents(
    ui: &mut egui::Ui,
    interchange: &mut Interchange,
    target: Option<&TeamEntry>,
    version: PesVersion,
    locked: bool,
    revision: u64,
) -> Option<Action> {
    let Some(team) = target else {
        ui.weak("Pick a team row to export or import.");
        return None;
    };
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        ui.weak(format!(
            "Export {} ({}) as ",
            display_name(&team.name),
            team.id
        ));
        if ui
            .button("Team TOML\u{2026}")
            .on_hover_text(
                "One team's full-fidelity file, the same text the CLI's export-toml writes",
            )
            .clicked()
        {
            action = Some(Action::Export(Export::TeamToml));
        }
        if ui
            .add_enabled(
                texport_layout(version).is_some(),
                egui::Button::new("Texport\u{2026}"),
            )
            .on_hover_text("A file PES itself can import")
            .on_disabled_hover_text("PES 2015 to 2017 have no Texport template")
            .clicked()
        {
            action = Some(Action::Export(Export::Texport));
        }
    });
    ui.separator();
    // One line, as the Comparator's pick row: the label, the file's name
    // truncated to the room the button leaves, the button at the right.
    ui.horizontal_top(|ui| {
        ui.weak(format!(
            "Import into {} ({}) ",
            display_name(&team.name),
            team.id
        ));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if ui
                .button("Pick\u{2026}")
                .on_hover_text("The file picked decides the format")
                .clicked()
            {
                action = Some(Action::Pick);
            }
            match &interchange.picked {
                Some(picked) => {
                    ui.add(
                        egui::Label::new(egui::RichText::new(file_name(&picked.path)).monospace())
                            .truncate()
                            .show_tooltip_when_elided(false),
                    )
                    .on_hover_text(picked.path.display().to_string());
                }
                None => {
                    ui.weak("(none)");
                }
            }
        });
    });
    let Some(picked) = &interchange.picked else {
        ui.weak(
            "Team TOML, .4ccs, .4cct and Texport files open here, and the file decides the format.",
        );
        return action;
    };
    let error = ui.visuals().error_fg_color;
    if let Err(refusal) = &picked.read {
        ui.colored_label(error, refusal);
        return action;
    }
    let staged = match &interchange.staged {
        Some((_, Ok(staged))) => staged,
        Some((_, Err(refusal))) => {
            ui.colored_label(error, refusal);
            return action;
        }
        None => return action,
    };
    ui.weak(intro(picked.format, staged.version, version));
    if !staged.pairing.is_empty() {
        ui.weak("Who gets which record");
        pairing_rows(ui, &staged.pairing);
    }
    let sections = &mut interchange.sections;
    ui.horizontal_wrapped(|ui| {
        ui.weak("Sections ");
        if staged.held.gameplay {
            checkbox(ui, &mut sections.gameplay, "Gameplay");
        }
        if staged.held.aesthetics {
            if locked {
                ui.add_enabled_ui(false, |ui| checkbox(ui, &mut false, "Aesthetics"))
                    .inner
                    .on_disabled_hover_text(OWNERSHIP_TIP);
            } else {
                checkbox(ui, &mut sections.aesthetics, "Aesthetics");
            }
        }
        if staged.held.tactics {
            checkbox(ui, &mut sections.tactics, "Tactics");
        }
    });
    match &staged.import {
        Err(refusal) => {
            ui.colored_label(error, refusal);
        }
        Ok(import) if !import.notes.is_empty() => {
            // A document naming no version is read as the save's.
            ui.weak(format!(
                "Import notes ({} file onto a {version} save)",
                staged.version.unwrap_or(version)
            ));
            notes(ui, &import.notes);
        }
        Ok(_) => {}
    }
    let held = staged.held;
    let any_chosen = (held.gameplay && sections.gameplay)
        || (held.aesthetics && sections.aesthetics && !locked)
        || (held.tactics && sections.tactics);
    // The chosen sections would leave the team as it is (its own file, or
    // one applied already): Apply is disabled and says so rather than
    // record an edit that changes nothing.
    let unchanged = any_chosen && matches!(&staged.import, Ok(import) if !import.changes);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                staged.import.is_ok() && any_chosen && !unchanged,
                egui::Button::new("Apply"),
            )
            .on_hover_text("Write the chosen sections as one undoable edit")
            .clicked()
        {
            action = Some(Action::Apply);
        }
        ui.weak("one undoable edit; the changes stay unsaved until Save.");
    });
    if interchange.applied == Some(revision) {
        ui.weak(format!(
            "Applied {}. The changes stay unsaved until Save.",
            file_name(&picked.path)
        ));
    } else if unchanged {
        ui.weak("The team already holds every value of the chosen sections.");
    }
    action
}

/// The intro line: the format, and the version the file was written from
/// against the save's.
fn intro(format: Format, from: Option<PesVersion>, save: PesVersion) -> String {
    let label = format.label();
    match from {
        None => format!("{label} naming no version, read as the save's."),
        Some(from) if from == save => format!("{label} written from a {from} save."),
        Some(from) => {
            format!("{label} written from a {from} save, converted to {save} where it can be.")
        }
    }
}

/// The height the pairing list and the note groups each take at most
/// before they scroll, so the rows around them and Apply under them stay
/// inside the window however long the file's lists are.
const LIST_HEIGHT: f32 = 240.0;

/// A squad file's records, one row each: the slot (weak), the record's
/// name, then the rostered player's shirt number (weak, monospace) and his
/// name in its colour codes, as the player list draws it.
fn pairing_rows(ui: &mut egui::Ui, rows: &[Pairing]) {
    let font = egui::TextStyle::Body.resolve(ui.style());
    egui::ScrollArea::vertical()
        .id_salt("save_editor_interchange_pairing")
        .max_height(LIST_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for row in rows {
                ui.horizontal(|ui| {
                    ui.weak(format!("{:02}", row.slot));
                    ui.label(&row.record);
                    ui.weak("\u{2192}");
                    ui.label(
                        egui::RichText::new(row.number.to_string())
                            .monospace()
                            .weak(),
                    );
                    let mut name = egui::text::LayoutJob::default();
                    append_name(&mut name, &row.name, &font);
                    ui.label(name);
                });
            }
        });
}

/// The notes in a scroll area as tall as the pairing list at most (the
/// draft's `#ix-warnings`): per code, in first-seen order, the catalog's
/// text as one weak line, then the code's groups. A team key's group is
/// its subject alone in the warning colour; a players' group is a
/// collapsed header `<subject> · <n> players` over one row per player.
fn notes(ui: &mut egui::Ui, groups: &[NoteGroup]) {
    let warn = ui.visuals().warn_fg_color;
    let mut codes: Vec<&str> = Vec::new();
    for group in groups {
        if !codes.contains(&group.code.as_str()) {
            codes.push(&group.code);
        }
    }
    egui::ScrollArea::vertical()
        .id_salt("save_editor_interchange_note_groups")
        .max_height(LIST_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for code in codes {
                let of_code: Vec<&NoteGroup> =
                    groups.iter().filter(|group| group.code == code).collect();
                ui.label(egui::RichText::new(&of_code[0].text).small().weak());
                for group in of_code {
                    let players = group
                        .notes
                        .iter()
                        .filter(|note| note.place.is_some())
                        .count();
                    if players == 0 {
                        ui.colored_label(warn, &group.subject);
                        continue;
                    }
                    let noun = if players == 1 { "player" } else { "players" };
                    collapsing_header(format!("{} \u{b7} {players} {noun}", group.subject))
                        .id_salt(("save_editor_interchange_notes", group.key.as_str()))
                        .default_open(false)
                        .show(ui, |ui| note_rows(ui, &group.notes));
                }
            }
        });
}

/// A players' group's rows: the place (weak), the player's name in its
/// colour codes as the pairing list draws it (`(empty place)` when the
/// place holds none), and the note's detail (weak).
fn note_rows(ui: &mut egui::Ui, notes: &[NoteRow]) {
    let font = egui::TextStyle::Body.resolve(ui.style());
    for note in notes {
        let Some(place) = note.place else {
            continue;
        };
        ui.horizontal(|ui| {
            ui.weak(format!("{place:02}"));
            match &note.name {
                Some(name) => {
                    let mut job = egui::text::LayoutJob::default();
                    append_name(&mut job, name, &font);
                    ui.label(job);
                }
                None => {
                    ui.weak("(empty place)");
                }
            }
            if !note.detail.is_empty() {
                ui.weak(&note.detail);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use pes_savefile::interchange::team_toml::ImportNote;
    use pes_savefile::{EditFile, fixture_save};
    use pes_version::PesVersion;

    use super::{note_row, note_subject};

    /// One note of each kind onto PES 19's team 701 (`/3/`), whose places
    /// 01 to 23 hold players of distinct names and 24 none: its row's place,
    /// the target's stored name at that place and the detail, and its
    /// subject.
    #[test]
    fn each_note_kind_reads_its_place_name_detail_and_subject() {
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes19)).expect("fixture save");
        let team = file.team(701).expect("team 701");
        assert_eq!(team.roster[23].player_id, 0, "premise: place 24 empty");
        let path = |path: &str| path.to_owned();
        let cases = [
            (
                ImportNote::NotEncodable {
                    path: path("players.03.positions.playing_style"),
                    label: "classic_no_10".to_owned(),
                },
                Some(3),
                Some("3DSMAX HAS STOPPED WORKING"),
                "Classic No. 10",
                "Playing style",
            ),
            (
                ImportNote::Capped {
                    path: path("players.05.appearance.motion.free_kick"),
                    from: 19,
                    to: 15,
                },
                Some(5),
                Some("HOW DO I?"),
                "19 \u{2192} 15",
                "Free kick motion",
            ),
            (
                ImportNote::TextCropped {
                    path: path("players.07.name"),
                    cropped: "JIGGLE PHY".to_owned(),
                },
                Some(7),
                Some("JIGGLE PHYSICS"),
                "JIGGLE PHY",
                "Name",
            ),
            (
                ImportNote::EmptySlot { slot: 24 },
                Some(24),
                None,
                "",
                "Empty roster place",
            ),
            (
                ImportNote::NotInThisVersion {
                    path: path("team.edit_flags.stadium"),
                },
                None,
                None,
                "",
                "Stadium edit flag",
            ),
            (
                ImportNote::NotEncodable {
                    path: path("tactics.preset_1.instructions.attack"),
                    label: "wing_back".to_owned(),
                },
                None,
                None,
                "Wing Back",
                "Preset 1 attacking instructions",
            ),
            (
                ImportNote::NotEncodable {
                    path: path("tactics.preset_2.instructions.defence"),
                    label: "wing_back".to_owned(),
                },
                None,
                None,
                "Wing Back",
                "Preset 2 defending instructions",
            ),
            (
                ImportNote::NotInThisVersion {
                    path: path("players.02.skills.skills.gk_long_throw"),
                },
                Some(2),
                Some("UNCANNY VALLEY"),
                "",
                "GK Long Throw",
            ),
            (
                ImportNote::NotInThisVersion {
                    path: path("players.02.skills.com_styles.mazing_run"),
                },
                Some(2),
                Some("UNCANNY VALLEY"),
                "",
                "Mazing Run",
            ),
        ];
        for (note, place, name, detail, subject) in cases {
            let row = note_row(&file, team, &note);
            assert_eq!(row.place, place, "{note:?}'s place");
            assert_eq!(row.name.as_deref(), name, "{note:?}'s name");
            assert_eq!(row.detail, detail, "{note:?}'s detail");
            assert_eq!(
                note_subject(PesVersion::Pes19, &note),
                subject,
                "{note:?}'s subject"
            );
        }
    }
}
