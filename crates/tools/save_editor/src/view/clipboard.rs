//! Ctrl+C and Ctrl+V, the draft's `clipCopy`/`clipPaste`: the editor's
//! own clipboard — at most one copy, a snapshot taken at Ctrl+C and
//! emptied when a save opens — acting on what was clicked last of three
//! kinds: the player list (a list row means the selected row, which Up
//! and Down move without a click), the Tactics preset selector (the
//! shown preset) or the formation selector (the shown formation). A
//! paste of another kind than the target's does nothing; each paste is
//! one sealed undo entry, writing exactly what its menu or dialog
//! writes. A copy also writes one line naming what it copied to the
//! system clipboard: the window hears Ctrl+V only as a paste of the
//! system's text, and an empty one sends nothing.

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::TeamToml;
use pes_savefile::model::names::display_name;
use pes_savefile::model::tactics::{Formation, TacticsPreset};
use pes_savefile::model::{PlayerEntry, TeamEntry};
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

/// The one copy the editor's clipboard holds: a snapshot taken at
/// Ctrl+C of what the last click named.
pub(super) enum Clip {
    /// A player's entry (for the stats `copy_stat_fields` moves).
    Player(Box<PlayerEntry>),
    /// A team's Gameplan: its `TeamToml::from_team` document reduced to
    /// the Gameplan section alone (`keep_sections`).
    Gameplan(Box<TeamToml>),
    /// One tactics preset.
    Preset(Box<TacticsPreset>),
    /// One formation.
    Formation(Box<Formation>),
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
/// a team row's paste writes, as the import panel's Gameplan choice.
const GAMEPLAN: Sections = Sections {
    stats: false,
    aesthetics: false,
    gameplan: true,
};

/// What Ctrl+C reads and Ctrl+V writes, resolved now: `List` reads the
/// selected row (the arrows move it without a click), the selector
/// kinds the selected row's team.
enum Target {
    /// The selected player's id.
    Player(u32),
    /// The selected team row's id.
    Team(u32),
    /// The shown preset on the selected row's team.
    Preset(u32),
    /// The shown formation on the selected row's team.
    Formation(u32),
}

/// The kind `view`'s last click names now; `None` with no save open, no
/// click recorded, or `List` with nothing selected.
fn target(view: &SaveView) -> Option<Target> {
    match view.click_kind? {
        ClickKind::List => match view.selected? {
            Selection::Player(..) => {
                selected_player(view).map(|(.., player_id)| Target::Player(player_id))
            }
            Selection::Team(team_id) => Some(Target::Team(team_id)),
        },
        ClickKind::Preset => selected_team_id(view).map(Target::Preset),
        ClickKind::Formation => selected_team_id(view).map(Target::Formation),
    }
}

/// Ctrl+C: the snapshot `click_kind` names, and one line naming what it
/// copied on the system clipboard — the window hears Ctrl+V only as a
/// paste of the system's text, and sends nothing for an empty one.
pub(super) fn copy(ctx: &egui::Context, view: &mut SaveView) {
    let Some(target) = target(view) else {
        return;
    };
    let Some(session) = view.session.as_ref() else {
        return;
    };
    let file = session.file();
    let (clip, named) = match target {
        Target::Player(player_id) => {
            let Some(player) = file.player(player_id) else {
                return;
            };
            (
                Clip::Player(Box::new(player.clone())),
                format!("the stats of {}", display_name(&player.name)),
            )
        }
        Target::Team(team_id) => {
            let Some(team) = file.team(team_id) else {
                return;
            };
            let Some(document) = gameplan_document(file, team) else {
                return;
            };
            (
                Clip::Gameplan(Box::new(document)),
                format!("the Gameplan of {}", display_name(&team.name)),
            )
        }
        Target::Preset(team_id) => {
            let Some(team) = file.team(team_id) else {
                return;
            };
            let preset = view.tactics.preset.min(2);
            (
                Clip::Preset(Box::new(team.tactics.presets[preset].clone())),
                format!("preset {} of {}", preset + 1, display_name(&team.name)),
            )
        }
        Target::Formation(team_id) => {
            let Some(team) = file.team(team_id) else {
                return;
            };
            let preset = view.tactics.preset.min(2);
            let formation = view.tactics.formation.min(2);
            (
                Clip::Formation(Box::new(
                    team.tactics.presets[preset].formations[formation].clone(),
                )),
                format!(
                    "the {} formation of {}",
                    FORMATIONS[formation],
                    display_name(&team.name)
                ),
            )
        }
    };
    view.clip = Some(clip);
    ctx.copy_text(format!("Unlimited Tool Works: {named}"));
}

/// Ctrl+V: the clipboard written over the target the last click names
/// now, each write exactly what its menu or dialog writes as one sealed
/// undo entry. A clipboard of another kind than the target's does
/// nothing.
pub(super) fn paste(view: &mut SaveView) {
    let Some(target) = target(view) else {
        return;
    };
    let SaveView {
        clip,
        session,
        tactics,
        ..
    } = view;
    let (Some(clip), Some(session)) = (clip.as_ref(), session.as_mut()) else {
        return;
    };
    match clip {
        Clip::Player(source) => {
            if let Target::Player(player_id) = target {
                session.copy_stats_from(player_id, source);
            }
        }
        Clip::Gameplan(document) => {
            if let Target::Team(team_id) = target {
                paste_gameplan(session, team_id, document);
            }
        }
        Clip::Preset(preset) => {
            if let Target::Preset(team_id) = target {
                session.paste_preset(team_id, tactics.preset.min(2), (**preset).clone());
            }
        }
        Clip::Formation(formation) => {
            if let Target::Formation(team_id) = target {
                session.paste_formation(
                    team_id,
                    tactics.preset.min(2),
                    tactics.formation.min(2),
                    (**formation).clone(),
                );
            }
        }
    }
}

/// `team`'s `TeamToml::from_team` document reduced to Gameplan alone,
/// as a copy onto the clipboard keeps it. A `from_team` refusal —
/// unreachable on a team of the save — is logged, not shown.
fn gameplan_document(file: &EditFile, team: &TeamEntry) -> Option<TeamToml> {
    let players: Vec<&PlayerEntry> = file.players().iter().collect();
    let mut document = match TeamToml::from_team(file.version(), team, &players) {
        Ok(document) => document,
        Err(error) => {
            log::debug!(
                "team {}'s Gameplan copy could not be written: {error}",
                team.id
            );
            return None;
        }
    };
    keep_sections(&mut document, GAMEPLAN);
    Some(document)
}

/// The Gameplan document's paste onto `team_id` (a team row's Ctrl+V):
/// `stage` onto the team, then the import panel's Apply for Gameplan
/// alone chosen — refused exactly when Apply is: a `stage` refusal
/// (unreachable on the save's own document, logged) or an import that
/// changes nothing.
fn paste_gameplan(session: &mut Session, team_id: u32, document: &TeamToml) {
    let Some(team) = session.file().team(team_id).cloned() else {
        return;
    };
    let import = match stage(session.file(), &team, document, GAMEPLAN) {
        Ok(import) => import,
        Err(error) => {
            log::debug!("the Gameplan paste onto team {team_id} was refused: {error}");
            return;
        }
    };
    // No gate on the write: a `from_team` document always holds the
    // Gameplan, and an import that changes nothing writes nothing
    // through `edit_entries`, which skips an unchanged entry.
    session.edit_entries(import.players, vec![import.team]);
}
