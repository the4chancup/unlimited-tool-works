//! The unsaved-changes prompt (SE-GUI-05, the draft's `drawUnsavedPrompt`):
//! "Unsaved changes" with Save, Discard and Cancel over the table of what a
//! close or an open would write or lose, met by the window's close or an
//! Open menu entry. The save's running-game question (`edit_mode_dialog`)
//! shares the module: the window's Save and Save as ask it first while the
//! save's own PES runs (SE-GUI-21's window rule).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use pes_savefile::model::PlayerEntry;
use pes_savefile::model::TeamEntry;
use pes_savefile::model::names::display_name;
use pes_savefile::model::player::STRIPPED_APPEARANCE_ID;
use pes_savefile::ops::compare::{PlayerDiff, TeamDiff};
use pes_savefile::schema::fields::RosterField;
use studio_core::RunningPes;
use studio_core::widgets::{
    FORM_ROW_SPACING, modal, modal_buttons, modal_frame, modal_sentence_gap,
};
use team_widgets::tactics::RosterRow;

use crate::session::Session;
use crate::session::selection::{self, TeamChoice};
use crate::view::{Refusal, SaveView, save, terms, write_now};

/// What the prompt does once answered.
#[derive(Debug)]
pub(crate) enum Pending {
    /// The window's close (`Tool::hold_close`): Discard drops the session
    /// and asks the shell to close again, so the re-asked `hold_close`
    /// returns `false`.
    Close,
    /// An Open menu entry: the save at the path opens.
    Open(PathBuf),
    /// The Open menu's `Choose file…`: the file dialog follows the answer,
    /// in the folder given when there is one (a cancelled dialog leaves
    /// the open save as it is).
    OpenPicker(Option<PathBuf>),
}

/// The prompt's state on `SaveView`: the pending action and the table's
/// rows, computed once when the prompt opens, never per frame.
pub(crate) struct Prompt {
    /// What the prompt does once answered.
    pending: Pending,
    /// The table's rows: `(item, label, old, new)` drawn as three
    /// columns, the Item cell `<item> · <label>` as the draft joins them.
    rows: Vec<(String, String, String, String)>,
}

/// The prompt's buttons.
#[derive(Debug)]
enum Answer {
    /// Writes the save first (the running-game question first when the
    /// save's own PES runs), then goes on.
    Save,
    /// Goes on without writing.
    Discard,
    /// Keeps everything.
    Cancel,
}

/// Opens the prompt for `pending` on a dirty session, or retargets one
/// already up (a window close asked while an Open prompt waits takes
/// its answer toward the close; the computed rows are the same either
/// way). The table's rows are computed now
/// (`Session::unsaved_changes`), never per frame — a codec error,
/// impossible on a save read whole, shows as a refusal rather than a
/// silently empty table.
pub(crate) fn open(view: &mut SaveView, pending: Pending) {
    let Some(session) = view.session.as_ref() else {
        return;
    };
    if let Some(prompt) = view.prompt.as_mut() {
        prompt.pending = pending;
        return;
    }
    match rows(session) {
        Ok(rows) => {
            view.prompt = Some(Prompt { pending, rows });
        }
        Err(error) => {
            view.refusal = Some(Refusal {
                title: "Cannot compare the save",
                message: format!("{error:#}"),
            });
        }
    }
}

/// The table's rows (the draft's `unsavedChanges`): the changed players
/// in the ALL list's order (a changed one the list does not show last by
/// id), a `Stripped` row after his field rows when his appearance id
/// changed (the field `compare_players` emits no row for), a roster
/// slot's `Shirt number` row under its player after his own (the dot's rule:
/// his dot covers his roster slot's number), then each changed team's
/// other rows in the open save's team order. The item is the player's
/// display name (`display_name`, colour codes dropped) or the team's.
/// A team row's slot-valued texts name the baseline's roster and records
/// on the old side, the current's on the new: a drag moves records
/// between the roster's ids while the stored byte still names the slot.
fn rows(session: &Session) -> anyhow::Result<Vec<(String, String, String, String)>> {
    let file = session.file();
    let version = session.version();
    let changes = session.unsaved_changes()?;
    let player_diffs: HashMap<u32, Vec<PlayerDiff>> = changes.players.into_iter().collect();
    let team_diffs: HashMap<u32, Vec<TeamDiff>> = changes.teams.into_iter().collect();
    // The ALL list runs to the thousands on a cup save: the id lookups
    // go through indexes built once, never `EditFile`'s linear scans
    // (`session`'s baseline lookups are already its own maps').
    let players: HashMap<u32, &PlayerEntry> = file
        .players()
        .iter()
        .map(|player| (player.id, player))
        .collect();
    let teams: HashMap<u32, &TeamEntry> = file.teams().iter().map(|team| (team.id, team)).collect();
    // The roster slots the ALL list shows (its order is the players'): a
    // `Shirt number` row of one goes under its player.
    let listed_rows = selection::rows(file, &TeamChoice::All, "");
    let listed: HashSet<(u32, u8)> = listed_rows
        .iter()
        .map(|row| (row.team_id, row.slot))
        .collect();
    let name_of = |id: u32| {
        players
            .get(&id)
            .map(|player| display_name(&player.name))
            .unwrap_or_else(|| id.to_string())
    };
    let mut rows = Vec::new();
    let mut shown: HashSet<u32> = HashSet::new();
    for entry in &listed_rows {
        // The item's text is looked up when a row names him, not per
        // listed row.
        let mut item = None;
        let mut named = || item.get_or_insert_with(|| name_of(entry.player_id)).clone();
        // A player rostered on two teams is listed per roster: his own
        // rows go under his first listing only, his `Shirt number` rows
        // stay per slot.
        if shown.insert(entry.player_id) {
            if let Some(diffs) = player_diffs.get(&entry.player_id) {
                for diff in diffs {
                    let term = terms::row_term(version, diff);
                    rows.push((named(), term.label, term.old, term.new));
                }
            }
            if let Some((label, old, new)) = stripped_term(
                session.baseline_player(entry.player_id),
                players.get(&entry.player_id).copied(),
            ) {
                rows.push((named(), label, old, new));
            }
        }
        let Some(team) = teams.get(&entry.team_id).copied() else {
            continue;
        };
        let Some(diffs) = team_diffs.get(&entry.team_id) else {
            continue;
        };
        let numbers: Vec<TeamDiff> = diffs
            .iter()
            .filter(|diff| {
                matches!(diff, TeamDiff::Roster { field: RosterField::Number(n), .. } if *n == entry.slot)
            })
            .cloned()
            .collect();
        if !numbers.is_empty() {
            // A team the diff names has a baseline (`unsaved_changes`
            // pairs them), so this cannot be `None`.
            let baseline_team = session
                .baseline_team(entry.team_id)
                .expect("a team diff implies a baseline");
            let old_names = roster_names(baseline_team, |id| session.baseline_player(id));
            let old_roster =
                roster_for(baseline_team, &old_names, |id| session.baseline_player(id));
            let new_names = roster_names(team, |id| players.get(&id).copied());
            let new_roster = roster_for(team, &new_names, |id| players.get(&id).copied());
            for term in terms::team_row_terms(
                version,
                team,
                baseline_team,
                &old_roster,
                &new_roster,
                &numbers,
            ) {
                rows.push((named(), term.label, term.old, term.new));
            }
        }
    }
    let mut unlisted: Vec<u32> = player_diffs
        .keys()
        .copied()
        .filter(|id| !shown.contains(id))
        .collect();
    unlisted.sort_unstable();
    for id in unlisted {
        let item = name_of(id);
        for diff in &player_diffs[&id] {
            let term = terms::row_term(version, diff);
            rows.push((item.clone(), term.label, term.old, term.new));
        }
        if let Some((label, old, new)) =
            stripped_term(session.baseline_player(id), players.get(&id).copied())
        {
            rows.push((item, label, old, new));
        }
    }
    for team in file.teams() {
        let Some(diffs) = team_diffs.get(&team.id) else {
            continue;
        };
        let remaining: Vec<TeamDiff> = diffs
            .iter()
            .filter(|diff| {
                !matches!(diff, TeamDiff::Roster { field: RosterField::Number(n), .. }
                    if listed.contains(&(team.id, *n)))
            })
            .cloned()
            .collect();
        if remaining.is_empty() {
            continue;
        }
        // A team the diff names has a baseline (`unsaved_changes` pairs
        // them), so this cannot be `None`.
        let baseline_team = session
            .baseline_team(team.id)
            .expect("a team diff implies a baseline");
        let old_names = roster_names(baseline_team, |id| session.baseline_player(id));
        let old_roster = roster_for(baseline_team, &old_names, |id| session.baseline_player(id));
        let new_names = roster_names(team, |id| players.get(&id).copied());
        let new_roster = roster_for(team, &new_names, |id| players.get(&id).copied());
        for term in terms::team_row_terms(
            version,
            team,
            baseline_team,
            &old_roster,
            &new_roster,
            &remaining,
        ) {
            rows.push((display_name(&team.name), term.label, term.old, term.new));
        }
    }
    Ok(rows)
}

/// `team`'s roster slots' display names for `roster_for` (the two steps
/// split so the `RosterRow`s can borrow them): `lookup` finds a rostered
/// id's record — the current file's index or `Session::baseline_player`.
pub(super) fn roster_names<'a>(
    team: &TeamEntry,
    lookup: impl Fn(u32) -> Option<&'a PlayerEntry>,
) -> Vec<String> {
    team.roster
        .iter()
        .map(|slot| {
            lookup(slot.player_id).map_or_else(String::new, |player| display_name(&player.name))
        })
        .collect()
}

/// `team`'s roster as the Tactics editor's `RosterRow`s: the display names
/// borrowed from `names` (`roster_names`' output), the raw names the
/// records' own, colour codes kept. `lookup` is the same one
/// `roster_names` ran under.
pub(super) fn roster_for<'a>(
    team: &'a TeamEntry,
    names: &'a [String],
    lookup: impl Fn(u32) -> Option<&'a PlayerEntry>,
) -> Vec<RosterRow<'a>> {
    team.roster
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let player = lookup(slot.player_id);
            RosterRow {
                name: names[index].as_str(),
                raw_name: player.map_or("", |player| player.name.as_str()),
                registered_position: player.map(|player| player.positions.registered),
                portrait: None,
                medal: None,
                overalls: None,
                rated: [false; 13],
                fielded: &[],
            }
        })
        .collect()
}

/// The `Stripped` row for a player whose appearance id differs from the
/// baseline's — `compare_players` emits no row for it, the Strip again
/// change (SE-GUI-20) being otherwise invisible in the table. "No" while
/// his id was set, "Yes" once at -1.
fn stripped_term(
    baseline: Option<&PlayerEntry>,
    player: Option<&PlayerEntry>,
) -> Option<(String, String, String)> {
    let (baseline, player) = (baseline?, player?);
    (player.appearance_id != baseline.appearance_id).then(|| {
        let yes_no = |stripped: bool| if stripped { "Yes" } else { "No" }.to_owned();
        (
            "Stripped".to_owned(),
            yes_no(baseline.appearance_id == Some(STRIPPED_APPEARANCE_ID)),
            yes_no(player.appearance_id == Some(STRIPPED_APPEARANCE_ID)),
        )
    })
}

/// The prompt while `view.prompt` holds one: the table and the three
/// answers. Cancel — the button, Esc or a click on the backdrop — closes
/// it and changes nothing; Discard goes on at once; Save writes first
/// and goes on only once the file is written (the running-game question
/// first when the save's own PES runs).
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>) {
    let Some(prompt) = view.prompt.as_ref() else {
        return;
    };
    let mut answer = None;
    let id = egui::Id::unique("save_editor_unsaved");
    let response = modal(ui.ctx(), id, "Unsaved changes", |ui| {
        // The modal is as wide as its widest row, at most nine tenths of
        // the window's width: `set_max_width` caps the content (the frame's
        // margins and stroke make up the rest), and a wider row scrolls
        // sideways inside the table, every cell on one line.
        let chrome = modal_frame(ui.style()).total_margin().sum().x;
        ui.set_max_width((ui.ctx().content_rect().width() * 0.9 - chrome).max(0.0));
        egui::ScrollArea::both().max_height(240.0).show(ui, |ui| {
            changes_table(ui, "unsaved changes", &prompt.rows, false);
        });
        modal_buttons(ui, |ui| {
            if ui.button("Cancel").clicked() {
                answer = Some(Answer::Cancel);
            }
            if ui.button("Discard").clicked() {
                answer = Some(Answer::Discard);
            }
            if ui.button("Save").clicked() {
                answer = Some(Answer::Save);
            }
        });
    });
    if response.should_close() {
        answer = Some(Answer::Cancel);
    }
    match answer {
        Some(Answer::Cancel) => view.prompt = None,
        Some(Answer::Discard) => {
            if let Some(prompt) = view.prompt.take() {
                go_on(view, prompt.pending);
            }
        }
        Some(Answer::Save) => {
            if save(view, running)
                && let Some(prompt) = view.prompt.take()
            {
                go_on(view, prompt.pending);
            }
        }
        None => {}
    }
}

/// The changes table (the draft's `changesTable`), the prompt's and the
/// patch panel's: a grid with the weak header row `Item`, `Old value`,
/// `New value`, then one row per `(item, label, old, new)`, the Item cell
/// `<item> · <label>` and the values in monospace, every cell on one line.
/// `selectable` makes each Item cell a click target selecting the row's
/// player; the index of the row whose Item cell was clicked.
pub(super) fn changes_table<'a>(
    ui: &mut egui::Ui,
    id: &str,
    rows: impl IntoIterator<Item = &'a (String, String, String, String)>,
    selectable: bool,
) -> Option<usize> {
    egui::Grid::new(id)
        .num_columns(3)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            fn cell(text: impl Into<egui::WidgetText>) -> egui::Label {
                egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend)
            }
            for head in ["Item", "Old value", "New value"] {
                ui.add(cell(egui::RichText::new(head).weak()));
            }
            ui.end_row();
            let mut clicked = None;
            for (index, (item, label, old, new)) in rows.into_iter().enumerate() {
                let item = cell(format!("{item} · {label}"));
                if selectable {
                    if ui
                        .add(item.sense(egui::Sense::click()))
                        .on_hover_text("Select this player")
                        .clicked()
                    {
                        clicked = Some(index);
                    }
                } else {
                    ui.add(item);
                }
                ui.add(cell(egui::RichText::new(old).monospace()));
                ui.add(cell(egui::RichText::new(new).monospace()));
                ui.end_row();
            }
            clicked
        })
        .inner
}

/// The pending action once Save or Discard answers it, or at once when
/// nothing is unsaved.
pub(super) fn go_on(view: &mut SaveView, pending: Pending) {
    match pending {
        // The session goes so the re-asked `hold_close` lets the window
        // close; `request_close` is `Tool::view`'s to send.
        Pending::Close => {
            view.session = None;
            view.request_close = true;
        }
        Pending::Open(path) => view.open(&path),
        Pending::OpenPicker(start) => {
            if let Some(path) = (view.open_dialog)(start.as_deref()) {
                view.open(&path);
            }
        }
    }
}

/// The running-game question `view.edit_mode_question` holds open
/// (SE-GUI-21's window rule): `<game> is running` asks the member to make
/// sure the game's Edit mode is closed. Save writes with his answer — a
/// prompt's pending action then goes on; Cancel, or a dismissal, writes
/// nothing and a prompt stays up.
pub(super) fn edit_mode_dialog(
    ui: &mut egui::Ui,
    view: &mut SaveView,
    running: &dyn Fn() -> Vec<RunningPes>,
) {
    let Some(game) = view
        .edit_mode_question
        .as_ref()
        .map(|question| question.game)
    else {
        return;
    };
    let mut write = false;
    let mut cancel = false;
    let id = egui::Id::unique("save_editor_edit_mode");
    let response = modal(ui.ctx(), id, format!("{game} is running"), |ui| {
        ui.label(
            "If the game's Edit mode is open, close it before saving. The game writes its own \
             copy of the save when you leave Edit mode, so a save written while it is open \
             would be lost.",
        );
        modal_sentence_gap(ui);
        ui.label("After saving, use the Load option in Edit mode to see the changes in game.");
        modal_buttons(ui, |ui| {
            if ui.button("Cancel").clicked() {
                cancel = true;
            }
            if ui.button("Save").clicked() {
                write = true;
            }
        });
    });
    if write {
        if let Some(question) = view.edit_mode_question.take() {
            write_now(view, running, &question.target, true);
        }
        // A written file unblocks the pending action (a failed write's
        // refusal keeps the prompt, as a cancelled question does).
        if view.refusal.is_none()
            && let Some(prompt) = view.prompt.take()
        {
            go_on(view, prompt.pending);
        }
    } else if cancel || response.should_close() {
        view.edit_mode_question = None;
    }
}
