//! Ctrl+C and Ctrl+V, the draft's `clipCopy`/`clipPaste`, acting on what
//! was clicked last of three kinds: the player list (a list row means the
//! selected row, which Up and Down move without a click), the Tactics
//! preset selector (the shown preset) or the formation selector (the
//! shown formation).
//!
//! The copy is text on the system clipboard, so it pastes into another
//! Save editor window, or into the same window after another save was
//! opened. The text is a Team TOML document holding only what was copied,
//! under two comment lines: the first names the copy for a person, the
//! second its kind for the paste. Team TOML rather than the records'
//! bytes, because the two windows may hold saves of different PES
//! versions: the document names its version, and its import converts the
//! values. Ctrl+V reads the text the window is handed (it hears Ctrl+V
//! only as a paste of the system clipboard's text) and writes it as the
//! import panel's Apply does, as one sealed undo entry. Text that is no
//! such document, or a copy of another kind than the target's, changes
//! nothing.

use std::collections::BTreeMap;

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::TeamToml;
use pes_savefile::model::PlayerEntry;
use pes_savefile::model::names::display_name;
use team_widgets::pitch::FORMATIONS;

use crate::session::Session;
use crate::session::import_sections::{Sections, keep_sections};
use crate::view::interchange::stage;
use crate::view::{SaveView, Selection, selected_player, selected_team_id};

/// The last clicked of the surfaces Ctrl+C and Ctrl+V read (the draft's
/// `clickKind`): a row of the player list, the Tactics preset selector
/// or the formation selector. Clicks elsewhere leave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClickKind {
    /// A player or team row of the list.
    List,
    /// The Tactics view's preset selector.
    Preset,
    /// The Tactics view's formation selector.
    Formation,
}

/// Whether a primary press happened over `response`'s rect this frame:
/// what the draft's `pointerdown` on a row or a selector means, where
/// `clicked()` waits for the release inside the widget and never fires
/// on a disabled one — the locked formation selector's press would set
/// no kind at all. `contains_pointer` is geometric, so it reports a
/// press on a disabled choice too.
pub(super) fn pressed(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.contains_pointer() && ui.input(|input| input.pointer.primary_pressed())
}

/// The Gameplan section alone chosen: what a team row's copy keeps and
/// a team row's paste writes, as the import panel's Gameplan choice. A
/// preset or formation paste writes this section too, its document
/// holding one preset or one formation of it.
const GAMEPLAN: Sections = Sections {
    stats: false,
    aesthetics: false,
    gameplan: true,
};

/// The Stats section alone chosen: what a player row's copy keeps and
/// its paste writes, the fields Copy stats moves.
const STATS: Sections = Sections {
    stats: true,
    aesthetics: false,
    gameplan: false,
};

/// The start of the clipboard text's first line, which names the copy.
const NAMING_LINE: &str = "# Unlimited Tool Works: ";

/// The start of the clipboard text's second line, which names its kind.
const KIND_LINE: &str = "# copied: ";

/// What a copy holds, as its text's second line names it: a paste writes
/// only onto a target of the same kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A player's stats.
    Stats,
    /// A team's Gameplan.
    Gameplan,
    /// One tactics preset.
    Preset,
    /// One formation.
    Formation,
}

impl Kind {
    const ALL: [Kind; 4] = [Kind::Stats, Kind::Gameplan, Kind::Preset, Kind::Formation];

    /// The word the kind line spells.
    fn label(self) -> &'static str {
        match self {
            Kind::Stats => "stats",
            Kind::Gameplan => "gameplan",
            Kind::Preset => "preset",
            Kind::Formation => "formation",
        }
    }
}

/// What Ctrl+C reads and Ctrl+V writes, resolved now: `List` reads the
/// selected row (the arrows move it without a click), the selector
/// kinds the selected row's team.
enum Target {
    /// The selected player's team id and roster slot (counting from 0).
    Player(u32, u8),
    /// The selected team row's id.
    Team(u32),
    /// The shown preset on the selected row's team.
    Preset(u32),
    /// The shown formation on the selected row's team.
    Formation(u32),
}

impl Target {
    /// The team the target belongs to.
    fn team_id(&self) -> u32 {
        match *self {
            Target::Player(team_id, _)
            | Target::Team(team_id)
            | Target::Preset(team_id)
            | Target::Formation(team_id) => team_id,
        }
    }

    /// The kind of copy the target takes and gives.
    fn kind(&self) -> Kind {
        match self {
            Target::Player(..) => Kind::Stats,
            Target::Team(_) => Kind::Gameplan,
            Target::Preset(_) => Kind::Preset,
            Target::Formation(_) => Kind::Formation,
        }
    }
}

/// The kind `view`'s last click names now; `None` with no save open, no
/// click recorded, or `List` with nothing selected.
fn target(view: &SaveView) -> Option<Target> {
    match view.click_kind? {
        ClickKind::List => match view.selected? {
            Selection::Player(..) => {
                selected_player(view).map(|(team_id, slot, _)| Target::Player(team_id, slot))
            }
            Selection::Team(team_id) => Some(Target::Team(team_id)),
        },
        ClickKind::Preset => selected_team_id(view).map(Target::Preset),
        ClickKind::Formation => selected_team_id(view).map(Target::Formation),
    }
}

/// Ctrl+C: what `click_kind` names, as its clipboard text on the system
/// clipboard.
pub(super) fn copy(ctx: &egui::Context, view: &SaveView) {
    let Some(target) = target(view) else {
        return;
    };
    let Some(session) = view.session.as_ref() else {
        return;
    };
    let shown = (view.tactics.preset.min(2), view.tactics.formation.min(2));
    let Some((named, document)) = copied(session.file(), &target, shown) else {
        return;
    };
    let text = match document.to_toml() {
        Ok(text) => text,
        Err(error) => {
            log::debug!("the copy of {named} could not be written: {error}");
            return;
        }
    };
    ctx.copy_text(format!(
        "{NAMING_LINE}{named}\n{KIND_LINE}{}\n{text}",
        target.kind().label()
    ));
}

/// The copy of `target` in `file`, `shown` the (preset, formation) the
/// Tactics view shows: the words naming it and the document holding it
/// alone. A player's stats are his `[players.NN]` table's Stats keys; a
/// Gameplan is the `[tactics]` table; a preset is `[tactics.preset_1]`
/// and a formation `[tactics.preset_1.formation_1]`, whichever was shown,
/// since the paste goes onto the one shown there. `None` when the target
/// names nothing the save holds; a `from_team` refusal, unreachable on a
/// team of the save, is logged, not shown.
fn copied(file: &EditFile, target: &Target, shown: (usize, usize)) -> Option<(String, TeamToml)> {
    let team = file.team(target.team_id())?;
    let players: Vec<&PlayerEntry> = file.players().iter().collect();
    let mut exported = match TeamToml::from_team(file.version(), team, &players) {
        Ok(document) => document,
        Err(error) => {
            log::debug!("team {}'s copy could not be written: {error}", team.id);
            return None;
        }
    };
    let team_name = display_name(&team.name);
    let (preset, formation) = shown;
    let mut document = TeamToml {
        pes_version: exported.pes_version,
        ..TeamToml::default()
    };
    let named = match *target {
        Target::Player(_, slot) => {
            let player = file.player(team.roster.get(usize::from(slot))?.player_id)?;
            keep_sections(&mut exported, STATS);
            // `[players.NN]` counts the roster slots from 1.
            let key = slot + 1;
            document.players = BTreeMap::from([(key, exported.players.remove(&key)?)]);
            format!("the stats of {}", display_name(&player.name))
        }
        Target::Team(_) => {
            document = exported;
            keep_sections(&mut document, GAMEPLAN);
            format!("the Gameplan of {team_name}")
        }
        Target::Preset(_) => {
            document.tactics.presets[0] = exported.tactics.presets[preset];
            format!("preset {} of {team_name}", preset + 1)
        }
        Target::Formation(_) => {
            document.tactics.presets[0].formations[0] =
                exported.tactics.presets[preset].formations[formation];
            format!("the {} formation of {team_name}", FORMATIONS[formation])
        }
    };
    Some((named, document))
}

/// Ctrl+V with `text` the system clipboard's: a copy of the target's kind
/// written onto the target the last click names now, as one sealed undo
/// entry. Any other text changes nothing.
pub(super) fn paste(view: &mut SaveView, text: &str) {
    let Some(target) = target(view) else {
        return;
    };
    let Some((kind, document)) = read(text) else {
        log::debug!("the pasted text is no copy of the Save editor's");
        return;
    };
    if kind != target.kind() {
        log::debug!(
            "a {} copy is not pasted onto a {} target",
            kind.label(),
            target.kind().label()
        );
        return;
    }
    let (preset, formation) = (view.tactics.preset.min(2), view.tactics.formation.min(2));
    let Some(session) = view.session.as_mut() else {
        return;
    };
    let mut placed = TeamToml {
        pes_version: document.pes_version,
        ..TeamToml::default()
    };
    let sections = match target {
        Target::Player(_, slot) => {
            // The copy's one player table, moved onto the target's slot.
            let mut tables = document.players.into_values();
            let (Some(table), None) = (tables.next(), tables.next()) else {
                log::debug!("the pasted stats copy holds no single player");
                return;
            };
            placed.players = BTreeMap::from([(slot + 1, table)]);
            STATS
        }
        Target::Team(_) => {
            placed = document;
            GAMEPLAN
        }
        Target::Preset(_) => {
            placed.tactics.presets[preset] = document.tactics.presets[0];
            GAMEPLAN
        }
        Target::Formation(_) => {
            placed.tactics.presets[preset].formations[formation] =
                document.tactics.presets[0].formations[0];
            GAMEPLAN
        }
    };
    import(session, target.team_id(), &placed, sections);
}

/// The kind and the document of a clipboard text a copy wrote: its
/// naming line, its kind line, then the Team TOML. `None` for any other
/// text; a document that does not parse is logged.
fn read(text: &str) -> Option<(Kind, TeamToml)> {
    let mut lines = text.splitn(3, '\n');
    if !lines.next()?.starts_with(NAMING_LINE) {
        return None;
    }
    // `trim_end`: a clipboard may hand the text back with Windows' line
    // endings.
    let label = lines.next()?.trim_end().strip_prefix(KIND_LINE)?;
    let kind = Kind::ALL.into_iter().find(|kind| kind.label() == label)?;
    match TeamToml::parse(lines.next().unwrap_or_default()) {
        Ok(document) => Some((kind, document)),
        Err(error) => {
            log::debug!("the pasted {label} copy does not read: {error}");
            None
        }
    }
}

/// `document`'s `sections` written onto `team_id` as the import panel's
/// Apply writes them: `stage` onto the team, then the team and its
/// rostered players as one sealed undo entry. A document from a save of
/// another PES version is converted by the import. A `stage` refusal (a
/// key the version cannot hold, in a document of its own version) is
/// logged, not shown.
fn import(session: &mut Session, team_id: u32, document: &TeamToml, sections: Sections) {
    let Some(team) = session.file().team(team_id).cloned() else {
        return;
    };
    let import = match stage(session.file(), &team, document, sections) {
        Ok(import) => import,
        Err(error) => {
            log::debug!("the paste onto team {team_id} was refused: {error}");
            return;
        }
    };
    // No gate on the write: an import that changes nothing writes
    // nothing through `edit_entries`, which skips an unchanged entry, so
    // a paste that changes nothing pushes no undo entry.
    session.edit_entries(import.players, vec![import.team]);
}
