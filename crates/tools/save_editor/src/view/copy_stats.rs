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
use pes_savefile::model::TeamEntry;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{
    FIELD_SPACING, add_enabled, combo_item, modal_buttons, step_choice, wheel_steps,
};

use crate::session::Session;
use crate::session::cup::cup_teams;
use crate::view::{SaveView, Selection};

/// How far the window's bottom edge first stands above the content
/// rect's bottom: the draft's `.modal-bg.floating` padding, clear of the
/// player list and the Ability stats column.
const BOTTOM_GAP: f32 = 56.0;

/// Copy and Swap's disabled hover while both rows name one player.
const SAME_PLAYER_TIP: &str = "The source and the destination are the same player";

/// One grid row's pick: the team and the roster slot on it. The player
/// is read off the roster each frame (`Pick::player_id`), so a roster
/// write under the open window shows and writes the slot's current
/// player, not the one the pick landed on.
#[derive(Clone, Copy)]
struct Pick {
    /// The picked team's id.
    team_id: u32,
    /// The picked slot in `team_id`'s roster (0-based).
    slot: u8,
}

impl Pick {
    /// The player `team_id`'s roster `slot` currently holds, `None` for a
    /// missing team, an empty slot or an id no record carries (the row
    /// then reads `?`).
    fn player_id(&self, session: &Session) -> Option<u32> {
        session.slot_player(self.team_id, self.slot)
    }
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
    /// The two combos' text widths and the session revision they were
    /// measured at: worked out once per revision over every cup team —
    /// measuring 4,600 players' labels each frame is too slow, and only
    /// a change may bring a wider name.
    pub(crate) widths: Option<(u64, (f32, f32))>,
}

impl CopySwap {
    /// The window as it opens: both rows on the selected player — on a
    /// team row, the team's first filled slot — so Copy and Swap start
    /// disabled until a pick or a list click moves one of them.
    pub(crate) fn new(session: &Session, selected: Selection) -> CopySwap {
        let pick = match selected {
            Selection::Player(team_id, slot) => Pick { team_id, slot },
            Selection::Team(team_id) => Pick {
                team_id,
                slot: first_filled_slot(session.file(), team_id).unwrap_or(0),
            },
        };
        CopySwap {
            source: pick,
            destination: pick,
            seen: Some(selected),
            line: String::new(),
            widths: None,
        }
    }

    /// The Team and Player combos' text widths, measured once for the
    /// session's current revision and kept: over every cup team the
    /// combos offer, so a pick or a team change never resizes the
    /// window's grid (the revision changing re-measures, an edit or a
    /// new save having changed the names on offer).
    fn widths(&mut self, ui: &egui::Ui, session: &Session) -> (f32, f32) {
        let revision = session.revision();
        match self.widths {
            Some((at, widths)) if at == revision => widths,
            _ => {
                let widths = measure_widths(ui, session.file());
                self.widths = Some((revision, widths));
                widths
            }
        }
    }
}

/// `team_id`'s first filled roster slot as `(index, player id)`,
/// skipping an id no player record carries (`None` for an empty or
/// missing team).
fn first_filled_at(file: &EditFile, team_id: u32) -> Option<(u8, u32)> {
    file.team(team_id).and_then(|team| {
        team.roster
            .iter()
            .enumerate()
            .find(|(_, slot)| slot.player_id != 0 && file.player(slot.player_id).is_some())
            .map(|(index, slot)| {
                (
                    u8::try_from(index).expect("a roster holds fewer than 256 slots"),
                    slot.player_id,
                )
            })
    })
}

/// `first_filled_at`'s slot index.
fn first_filled_slot(file: &EditFile, team_id: u32) -> Option<u8> {
    first_filled_at(file, team_id).map(|(slot, _)| slot)
}

/// The first filled roster slot's player id on `team_id`
/// (`first_filled_at`'s), `None` for an empty or missing team.
pub(crate) fn first_filled(file: &EditFile, team_id: u32) -> Option<u32> {
    first_filled_at(file, team_id).map(|(_, id)| id)
}

/// The player combo's `(slot, label)` options for `team_id`: every
/// filled roster slot's index and label in slot order, each labeled by
/// its place in the roster (`01` to `23`, not the shirt number),
/// registered position and display name, as the player list shows them.
fn roster_options(file: &EditFile, team_id: u32) -> Vec<(u8, String)> {
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
        options.push((
            u8::try_from(index).expect("a roster holds fewer than 256 slots"),
            label,
        ));
    }
    options
}

/// `player_id`'s display name for the result line (the id as a fallback).
fn player_name(file: &EditFile, player_id: u32) -> String {
    file.player(player_id)
        .map(|player| display_name(&player.name))
        .unwrap_or_else(|| player_id.to_string())
}

/// The widest team name and the widest roster label of the cup's
/// teams, as laid out at the Button font: what the two combos size
/// themselves by, so each keeps one width whatever team its row names.
fn measure_widths(ui: &egui::Ui, file: &EditFile) -> (f32, f32) {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_width = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::PLACEHOLDER)
            .size()
            .x
    };
    let teams: Vec<&TeamEntry> = cup_teams(file).collect();
    let team = teams
        .iter()
        .map(|entry| text_width(&display_name(&entry.name)))
        .fold(0.0_f32, f32::max);
    let player = teams
        .iter()
        .flat_map(|entry| roster_options(file, entry.id))
        .map(|(_, label)| text_width(&label))
        .fold(0.0_f32, f32::max);
    (team, player)
}

/// One grid row's controls: `label` in the first column, the team combo
/// in the second (the cup teams by display name) and the player combo in
/// the third (the picked team's filled roster slots). A changed team
/// takes its first filled slot; both combos keep the wheel stepping.
/// Each combo is as wide as its widest option plus the combo's chrome, so
/// a pick never resizes the window's grid.
fn pick_row(
    ui: &mut egui::Ui,
    session: &Session,
    salt: &str,
    label: &str,
    pick: &mut Pick,
    widths: (f32, f32),
) {
    ui.label(label);
    let file = session.file();
    // The closed combo's chrome around its text: the button's padding on
    // both sides, the gap and the arrow.
    let chrome = {
        let spacing = ui.spacing();
        2.0 * spacing.button_padding.x + spacing.icon_spacing + spacing.icon_width
    };
    let mut team = pick.team_id;
    let combo = egui::ComboBox::from_id_salt((salt, "team"))
        .selected_text(
            file.team(team)
                .map(|entry| display_name(&entry.name))
                .unwrap_or_else(|| team.to_string()),
        )
        .width(widths.0 + chrome)
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
        pick.slot = first_filled_slot(file, team).unwrap_or(pick.slot);
    }
    let options = roster_options(file, pick.team_id);
    let mut slot = pick.slot;
    let combo = egui::ComboBox::from_id_salt((salt, "player"))
        .selected_text(
            options
                .iter()
                .find(|(index, _)| *index == slot)
                .map(|(_, label)| label.clone())
                .unwrap_or_else(|| "?".to_owned()),
        )
        .width(widths.1 + chrome)
        .show_ui(ui, |ui| {
            for (index, label) in &options {
                combo_item(ui, &mut slot, *index, label);
            }
        })
        .response;
    let slots = options.iter().map(|(index, _)| *index);
    if let Some(next) = step_choice(slots, &slot, wheel_steps(ui, &combo)) {
        slot = next;
    }
    pick.slot = slot;
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
                if session.slot_player(team_id, slot).is_some() {
                    dialog.destination = Pick { team_id, slot };
                }
            }
            Some(Selection::Team(team_id)) => {
                if let Some(slot) = first_filled_slot(session.file(), team_id) {
                    dialog.destination = Pick { team_id, slot };
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
            let widths = dialog.widths(ui, session);
            let grid = egui::Grid::new("save_editor_copy_stats_grid")
                .num_columns(3)
                .spacing(egui::vec2(ui.spacing().item_spacing.x, FIELD_SPACING))
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
                        widths,
                    );
                    ui.end_row();
                    pick_row(
                        ui,
                        session,
                        "save_editor_copy_stats_destination",
                        "Destination",
                        &mut dialog.destination,
                        widths,
                    );
                    ui.end_row();
                });
            // The last action's result: a line's room is always reserved
            // so the window does not jump when it first lands, and the
            // line is cut at the grid's edge so a long one does not widen
            // the window either.
            let line_height = ui.text_style_height(&egui::TextStyle::Body);
            ui.scope(|ui| {
                ui.set_min_height(line_height);
                ui.set_max_width(grid.response.rect.width());
                ui.add(
                    egui::Label::new(egui::RichText::new(dialog.line.as_str()).weak()).truncate(),
                );
            });
            // The picks name roster slots; the players written are the
            // ones the slots hold this frame, 0 for an unresolved pick
            // (`copy_stats`/`swap_stats` ignore one).
            let source = dialog.source.player_id(session).unwrap_or(0);
            let destination = dialog.destination.player_id(session).unwrap_or(0);
            let same = source == destination;
            modal_buttons(ui, |ui| {
                if ui.button("Close").clicked() {
                    close = true;
                }
                if add_enabled(ui, !same, egui::Button::new("Swap"))
                    .on_disabled_hover_text(SAME_PLAYER_TIP)
                    .clicked()
                {
                    session.swap_stats(source, destination);
                    let source = player_name(session.file(), source);
                    let destination = player_name(session.file(), destination);
                    dialog.line = format!("Swapped the stats of {source} and {destination}.");
                }
                if add_enabled(ui, !same, egui::Button::new("Copy"))
                    .on_disabled_hover_text(SAME_PLAYER_TIP)
                    .clicked()
                {
                    session.copy_stats(destination, source);
                    let source = player_name(session.file(), source);
                    let destination = player_name(session.file(), destination);
                    dialog.line = format!("Copied {source}'s stats to {destination}.");
                }
            });
        });
    if close || !open {
        view.copy_swap = None;
    }
}
