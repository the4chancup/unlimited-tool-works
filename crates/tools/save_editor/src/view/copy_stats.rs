//! The Copy/swap stats window (the draft's `drawCopyStats`): the Player
//! menu's one stat-moving entry, an `egui::Window` — not a modal — that
//! stays open over a usable view while the member works through a run of
//! destinations. The source is picked once in its dropdowns and the
//! list's selection picks each destination, so copying one player's
//! stats onto several is a click per player instead of a reopened
//! dialog per player. The operations are `Session::copy_stats` and
//! `Session::swap_stats` (the parity row's field set), each one sealed
//! undo entry, neither closing the window.

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{add_enabled, combo_item, modal_buttons, step_choice, wheel_steps};

use crate::session::Session;
use crate::session::cup::cup_teams;
use crate::view::{SaveView, Selection};

/// How far the window's bottom edge first stands above the content
/// rect's bottom: the draft's `.modal-bg.floating` padding, clear of the
/// player list and the Ability stats column.
const BOTTOM_GAP: f32 = 56.0;

/// Copy and Swap's disabled hover while both rows name one player.
const SAME_PLAYER_TIP: &str = "The source and the destination are the same player";

/// One grid row's pick: the team and the player on its roster.
#[derive(Clone, Copy)]
struct Pick {
    /// The picked team's id.
    team_id: u32,
    /// The picked player's id, a filled slot of `team_id`.
    player_id: u32,
}

/// The Copy/swap stats window's state (`view.copy_swap` while it is
/// open): the Source and Destination rows' picks, the last action's
/// line, and the selection the destination last followed — each player
/// picked by team, then by roster slot (one list of a cup save's 4,600
/// players cannot be browsed).
pub(crate) struct CopySwap {
    /// The Source row's pick.
    source: Pick,
    /// The Destination row's pick.
    destination: Pick,
    /// The list's selection as the window last saw it: a change on the
    /// next frame is the new destination.
    seen: Option<Selection>,
    /// The last Copy/Swap's result, the weak line under the grid; empty
    /// until the first action.
    line: String,
}

impl CopySwap {
    /// The window as it opens: both rows on the selected player — on a
    /// team row, the team's first filled slot — so Copy and Swap start
    /// disabled until a pick or a list click moves one of them.
    pub(crate) fn new(session: &Session, selected: Option<Selection>) -> CopySwap {
        let pick = match selected {
            Some(Selection::Player(team_id, slot)) => Pick {
                team_id,
                player_id: session.slot_player(team_id, slot).unwrap_or(0),
            },
            Some(Selection::Team(team_id)) => Pick {
                team_id,
                player_id: first_filled(session.file(), team_id).unwrap_or(0),
            },
            None => Pick {
                team_id: 0,
                player_id: 0,
            },
        };
        CopySwap {
            source: pick,
            destination: pick,
            seen: selected,
            line: String::new(),
        }
    }
}

/// The first filled roster slot's player id on `team_id`, skipping an id
/// no player record carries (`None` for an empty or missing team).
pub(crate) fn first_filled(file: &EditFile, team_id: u32) -> Option<u32> {
    file.team(team_id).and_then(|team| {
        team.roster
            .iter()
            .map(|slot| slot.player_id)
            .find(|&id| id != 0 && file.player(id).is_some())
    })
}

/// The player combo's `(player_id, label)` options for `team_id`: every
/// filled roster slot in slot order, each labeled by its place in the
/// roster (`01` to `23`, not the shirt number), registered position and
/// display name, as the player list shows them.
fn roster_options(file: &EditFile, team_id: u32) -> Vec<(u32, String)> {
    let mut options = Vec::new();
    let Some(team) = file.team(team_id) else {
        return options;
    };
    for (index, slot) in team.roster.iter().enumerate() {
        if slot.player_id == 0 {
            continue;
        }
        let place = format!("{:02}", index + 1);
        let label = match file.player(slot.player_id) {
            Some(player) => {
                let position = POSITIONS
                    .get(usize::from(player.positions.registered))
                    .copied()
                    .unwrap_or("?");
                format!("{place} {position} {}", display_name(&player.name))
            }
            None => format!("{place} ?"),
        };
        options.push((slot.player_id, label));
    }
    options
}

/// `player_id`'s display name for the result line (the id as a fallback).
fn player_name(file: &EditFile, player_id: u32) -> String {
    file.player(player_id)
        .map(|player| display_name(&player.name))
        .unwrap_or_else(|| player_id.to_string())
}

/// One grid row's controls: `label` in the first column, the team combo
/// in the second (the cup teams by display name) and the player combo in
/// the third (the picked team's filled roster slots). A changed team
/// takes its first filled slot; both combos keep the wheel stepping.
fn pick_row(ui: &mut egui::Ui, session: &Session, salt: &str, label: &str, pick: &mut Pick) {
    ui.label(label);
    let file = session.file();
    let mut team = pick.team_id;
    let combo = egui::ComboBox::from_id_salt((salt, "team"))
        .selected_text(
            file.team(team)
                .map(|entry| display_name(&entry.name))
                .unwrap_or_else(|| team.to_string()),
        )
        .show_ui(ui, |ui| {
            for entry in cup_teams(file) {
                combo_item(ui, &mut team, entry.id, display_name(&entry.name));
            }
        })
        .response;
    let teams = cup_teams(file).map(|entry| entry.id);
    if let Some(next) = step_choice(teams, &team, wheel_steps(ui, &combo)) {
        team = next;
    }
    if team != pick.team_id {
        pick.team_id = team;
        // The new team's first filled slot is the new pick.
        pick.player_id = first_filled(file, team).unwrap_or(pick.player_id);
    }
    let options = roster_options(file, pick.team_id);
    let mut player = pick.player_id;
    let combo = egui::ComboBox::from_id_salt((salt, "player"))
        .selected_text(
            options
                .iter()
                .find(|(id, _)| *id == player)
                .map(|(_, label)| label.clone())
                .unwrap_or_else(|| "?".to_owned()),
        )
        .show_ui(ui, |ui| {
            for (id, label) in &options {
                combo_item(ui, &mut player, *id, label);
            }
        })
        .response;
    let players = options.iter().map(|(id, _)| *id);
    if let Some(next) = step_choice(players, &player, wheel_steps(ui, &combo)) {
        player = next;
    }
    pick.player_id = player;
}

/// The Copy/swap stats window while `view.copy_swap` holds one (the
/// draft's `drawCopyStats` over `.modal-bg.floating`): a non-modal
/// `egui::Window` titled `Copy/swap stats`, first centred horizontally
/// `BOTTOM_GAP` over the content rect's bottom, then where the member
/// leaves it (egui keeps a window's dragged position; the title bar's
/// close button comes from `open` as on the Live AATF window). The grid
/// is two rows by two dropdown columns under the weak Team/Player
/// heads; the weak line under it is always one line tall so the window
/// does not jump when a result lands. While it is open, a list
/// selection that changed since the last frame — a click on another
/// row, or Up/Down — sets the Destination row; the Source row never
/// follows the list. Copy and Swap apply their `Session` operation
/// without closing, disabled while both rows name one player; Close and
/// the title bar's button close.
pub(crate) fn window(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.copy_swap.is_none() {
        return;
    }
    let selected = view.selected;
    let (Some(session), Some(dialog)) = (view.session.as_mut(), view.copy_swap.as_mut()) else {
        return;
    };
    if dialog.seen != selected {
        dialog.seen = selected;
        match selected {
            Some(Selection::Player(team_id, slot)) => {
                if let Some(player_id) = session.slot_player(team_id, slot) {
                    dialog.destination = Pick { team_id, player_id };
                }
            }
            Some(Selection::Team(team_id)) => {
                if let Some(player_id) = first_filled(session.file(), team_id) {
                    dialog.destination = Pick { team_id, player_id };
                }
            }
            None => {}
        }
    }
    let mut open = true;
    let mut close = false;
    let screen = ui.ctx().content_rect();
    egui::Window::new(studio_core::theme::window_title_text("Copy/swap stats"))
        .id(egui::Id::unique("save_editor_copy_stats"))
        .pivot(egui::Align2::CENTER_BOTTOM)
        .default_pos(egui::pos2(screen.center().x, screen.bottom() - BOTTOM_GAP))
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            egui::Grid::new("save_editor_copy_stats_grid")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("");
                    ui.weak("Team");
                    ui.weak("Player");
                    ui.end_row();
                    pick_row(
                        ui,
                        session,
                        "save_editor_copy_stats_source",
                        "Source",
                        &mut dialog.source,
                    );
                    ui.end_row();
                    pick_row(
                        ui,
                        session,
                        "save_editor_copy_stats_destination",
                        "Destination",
                        &mut dialog.destination,
                    );
                    ui.end_row();
                });
            // The last action's result: a line's room is always reserved
            // so the window does not jump when it first lands.
            let line_height = ui.text_style_height(&egui::TextStyle::Body);
            ui.scope(|ui| {
                ui.set_min_height(line_height);
                ui.weak(dialog.line.as_str());
            });
            let same = dialog.source.player_id == dialog.destination.player_id;
            modal_buttons(ui, |ui| {
                if ui.button("Close").clicked() {
                    close = true;
                }
                if add_enabled(ui, !same, egui::Button::new("Swap"))
                    .on_disabled_hover_text(SAME_PLAYER_TIP)
                    .clicked()
                {
                    session.swap_stats(dialog.source.player_id, dialog.destination.player_id);
                    let source = player_name(session.file(), dialog.source.player_id);
                    let destination = player_name(session.file(), dialog.destination.player_id);
                    dialog.line = format!("Swapped the stats of {source} and {destination}.");
                }
                if add_enabled(ui, !same, egui::Button::new("Copy"))
                    .on_disabled_hover_text(SAME_PLAYER_TIP)
                    .clicked()
                {
                    session.copy_stats(dialog.destination.player_id, dialog.source.player_id);
                    let source = player_name(session.file(), dialog.source.player_id);
                    let destination = player_name(session.file(), dialog.destination.player_id);
                    dialog.line = format!("Copied {source}'s stats to {destination}.");
                }
            });
        });
    if close || !open {
        view.copy_swap = None;
    }
}
