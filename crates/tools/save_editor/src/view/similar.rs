//! The Select all similar players window (the draft's `dlgSimilar`):
//! opened from the button heading the quick actions' grid on the
//! selected player, it counts the players sharing with him what its
//! boxes ask — the registered position, the cards, the ability stats —
//! in his team or in every cup team, and Select makes them the group
//! (`view.group`), so a tier's players can be edited as one.

use std::collections::HashMap;

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use pes_savefile::model::{PlayerEntry, TeamEntry};
use studio_core::widgets::{
    CHECKBOX_SPACING, add_enabled, checkbox, modal, modal_buttons, radio_value,
};
use team_widgets::names::{append_name, plain_format};

use crate::session::cup::cup_teams;
use crate::session::selection::{TeamCategory, TeamChoice};
use crate::view::{SaveView, Selection};

/// Where the window's match looks (the draft's `Look in` radios): the
/// player's team alone, or every cup team.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    /// The anchor player's team.
    Team,
    /// `Every team`: every cup team's roster, whatever the category.
    All,
}

/// The window's three boxes and what each compares (the draft's
/// `SIMILAR_BOXES`): `Position` the registered position, `Cards` the
/// skills and the COM playing styles both, `Stats` the ability stats
/// (the 25 Set stats writes). All ticked when the window opens; an
/// unticked box leaves its fields out of the match.
#[derive(Clone, Copy)]
pub(super) struct Boxes {
    /// `Position`: `positions.registered`.
    pub(super) position: bool,
    /// `Cards`: `skills.skills` and `skills.com_styles` both.
    pub(super) cards: bool,
    /// `Stats`: `stats.abilities()`.
    pub(super) stats: bool,
}

/// The window's state while it is open (`view.similar`): the player it
/// was opened on, the boxes, the scope and the current match. Nothing is
/// kept between openings: `open` starts every box ticked and his team.
pub(super) struct Similar {
    /// The player the window matches on, `(team id, roster slot)`.
    anchor: (u32, u8),
    /// The boxes' ticks.
    boxes: Boxes,
    /// `Look in`'s pick.
    scope: Scope,
    /// The rows `boxes` and `scope` match, the anchor first: the Select
    /// button's N and its write to `view.group`. Run at open and on each
    /// box or radio change, never a frame (Every team covers about 4,600
    /// players).
    matches: Vec<(u32, u8)>,
}

impl Similar {
    /// The window opened on the selected player `(team id, roster
    /// slot)`: the three boxes ticked, his team looked in, the match
    /// already run.
    pub(super) fn open(file: &EditFile, anchor: (u32, u8)) -> Similar {
        let mut similar = Similar {
            anchor,
            boxes: Boxes {
                position: true,
                cards: true,
                stats: true,
            },
            scope: Scope::Team,
            matches: Vec::new(),
        };
        similar.rematch(file);
        similar
    }

    /// `matches` under the boxes and scope as they stand.
    fn rematch(&mut self, file: &EditFile) {
        self.matches = matching(file, self.anchor, self.boxes, self.scope);
    }
}

/// The player rows of `scope` whose player equals the anchor's on each
/// ticked box: `position` on `positions.registered`, `cards` on
/// `skills.skills` and `skills.com_styles` both, `stats` on
/// `stats.abilities()`; no box ticked matches every rostered row. The
/// anchor's row first — he always matches — then the rest in list
/// order: the team's roster order under `Scope::Team`, every cup team's
/// in record order under `Scope::All` (a player on two cup teams is two
/// rows). Empty slots and slots whose id no player record carries are
/// skipped.
pub(super) fn matching(
    file: &EditFile,
    anchor: (u32, u8),
    boxes: Boxes,
    scope: Scope,
) -> Vec<(u32, u8)> {
    let Some(anchor_entry) = file
        .team(anchor.0)
        .and_then(|team| team.roster.get(usize::from(anchor.1)))
        .and_then(|slot| file.player(slot.player_id))
    else {
        return Vec::new();
    };
    // Player ids are stable inside an open file: one map serves the
    // whole scan instead of a `file.player()` linear scan per slot.
    let players: HashMap<u32, &PlayerEntry> = file
        .players()
        .iter()
        .map(|player| (player.id, player))
        .collect();
    let mut matches = vec![anchor];
    let mut add = |team: &TeamEntry| {
        for (slot, roster) in team.roster.iter().enumerate() {
            let Some(&player) = players.get(&roster.player_id) else {
                continue;
            };
            let slot = u8::try_from(slot).expect("a roster is at most 40 slots");
            if (team.id, slot) != anchor && alike(player, anchor_entry, boxes) {
                matches.push((team.id, slot));
            }
        }
    };
    match scope {
        Scope::Team => {
            if let Some(team) = file.team(anchor.0) {
                add(team);
            }
        }
        Scope::All => cup_teams(file).for_each(add),
    }
    matches
}

/// Whether `player` equals `anchor` on every ticked box.
fn alike(player: &PlayerEntry, anchor: &PlayerEntry, boxes: Boxes) -> bool {
    (!boxes.position || player.positions.registered == anchor.positions.registered)
        && (!boxes.cards
            || player.skills.skills == anchor.skills.skills
                && player.skills.com_styles == anchor.skills.com_styles)
        && (!boxes.stats || player.stats.abilities() == anchor.stats.abilities())
}

/// The window while `view.similar` holds one (the draft's `dlgSimilar`),
/// titled `Select all similar players`: `Match <name> on:` (the name's
/// colour codes drawn), the three boxes, `Look in` and the radios on one
/// row — the player's team by its display name, `Every team` — then
/// `Select N players` (N counting him; `No other player matches`,
/// disabled, when he alone matches) and Cancel. Escape or Cancel closes
/// it, changing nothing. Select makes the match the group and him the
/// selection; after `Every team` the selector moves to `All teams` and
/// the filter is cleared, so every selected row is in the list (a
/// category that leaves one of their teams out widens to `All`, as for
/// a selection made elsewhere).
pub(super) fn dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
        session,
        similar,
        group,
        selected,
        team,
        team_category,
        filter,
        ..
    } = view;
    let (Some(session), Some(similar)) = (session.as_ref(), similar.as_mut()) else {
        return;
    };
    let file = session.file();
    let anchor = similar.anchor;
    let Some(team_entry) = file.team(anchor.0) else {
        return;
    };
    let Some(name) = team_entry
        .roster
        .get(usize::from(anchor.1))
        .and_then(|slot| file.player(slot.player_id))
        .map(|player| player.name.clone())
    else {
        return;
    };
    let team_name = display_name(&team_entry.name);
    let mut close = false;
    let mut select = false;
    let id = egui::Id::unique("save_editor_similar");
    let response = modal(ui.ctx(), id, "Select all similar players", |ui| {
        // `Match <name> on:` — the name's colour codes drawn, as the
        // view's heading draws them.
        let body = egui::TextStyle::Body.resolve(ui.style());
        let mut line = egui::text::LayoutJob::default();
        line.append("Match ", 0.0, plain_format(body.clone()));
        append_name(&mut line, &name, &body);
        line.append(" on:", 0.0, plain_format(body));
        ui.label(line);
        // The three boxes, the draft's rows a `CHECKBOX_SPACING` apart.
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = CHECKBOX_SPACING;
            let changed = checkbox(
                ui,
                &mut similar.boxes.position,
                box_label(ui, "Position", "registered"),
            )
            .changed()
                | checkbox(
                    ui,
                    &mut similar.boxes.cards,
                    box_label(ui, "Cards", "skills, COM playing styles"),
                )
                .changed()
                | checkbox(
                    ui,
                    &mut similar.boxes.stats,
                    box_label(ui, "Stats", "ability stats"),
                )
                .changed();
            if changed {
                similar.rematch(file);
            }
        });
        // `Look in` and its radios on one row (the draft's `.row`).
        ui.horizontal(|ui| {
            ui.label("Look in");
            let changed = radio_value(ui, &mut similar.scope, Scope::Team, team_name.as_str())
                .changed()
                | radio_value(ui, &mut similar.scope, Scope::All, "Every team").changed();
            if changed {
                similar.rematch(file);
            }
        });
        modal_buttons(ui, |ui| {
            if ui.button("Cancel").clicked() {
                close = true;
            }
            let count = similar.matches.len();
            let (label, enabled) = if count < 2 {
                ("No other player matches".to_owned(), false)
            } else {
                (format!("Select {count} players"), true)
            };
            if add_enabled(ui, enabled, egui::Button::new(label)).clicked() {
                select = true;
                close = true;
            }
        });
    });
    if select {
        group.clone_from(&similar.matches);
        *selected = Some(Selection::Player(anchor.0, anchor.1));
        if similar.scope == Scope::All {
            *team = TeamChoice::All;
            // The row the click selects would be hidden by a filter
            // matching nothing; a category that leaves one of their
            // teams out hides the rest (as `select_team` widens it).
            filter.clear();
            if group
                .iter()
                .any(|&(team_id, _)| !team_category.contains(team_id))
            {
                *team_category = TeamCategory::All;
            }
        }
    }
    if close || response.should_close() {
        view.similar = None;
    }
}

/// A box's label (the draft's `.cb` rows): `label`, then ` (hint)` in
/// the weak colour.
fn box_label(ui: &egui::Ui, label: &str, hint: &str) -> egui::text::LayoutJob {
    let body = egui::TextStyle::Body.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    job.append(&format!("{label} "), 0.0, plain_format(body.clone()));
    job.append(
        &format!("({hint})"),
        0.0,
        egui::TextFormat::simple(body, ui.visuals().weak_text_color()),
    );
    job
}
