//! The unsaved-changes prompt (SE-GUI-05, the draft's `drawUnsavedPrompt`):
//! "Unsaved changes" with Save, Discard and Cancel over the table of what a
//! close or an open would write or lose, met by the window's close, Close
//! save or one of the File menu's opens. The save's running-game question
//! (`edit_mode_dialog`) shares the module: the window's Save and Save as
//! ask it first while the save's own PES runs (SE-GUI-21's window rule).
//! So does `confirm`, the question a dialog asks before it replaces or
//! removes a file.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use pes_savefile::model::PlayerEntry;
use pes_savefile::model::TeamEntry;
use pes_savefile::model::names::display_name;
use pes_savefile::ops::compare::{PlayerDiff, TeamDiff};
use pes_savefile::schema::fields::RosterField;
use studio_core::RunningPes;
use studio_core::widgets::{
    TEXT_ROW_SPACING, modal, modal_buttons, modal_frame, modal_sentence_gap,
};
use team_widgets::tactics::RosterRow;

use crate::session::selection::{self, TeamCategory, TeamChoice};
use crate::session::{MovedPlace, PutBack, Session};
use crate::view::terms::{self, Term};
use crate::view::{Refusal, SaveView, following_selection, info_strip, save, write_now};

/// A confirmation modal: `title`, `text`, then `accept` and Cancel. The
/// answer, `None` while it waits; closing it is Cancel.
pub(super) fn confirm(
    ui: &egui::Ui,
    id: &str,
    title: &str,
    text: &str,
    accept: &str,
) -> Option<bool> {
    let mut answer = None;
    let response = modal(ui.ctx(), egui::Id::unique(id), title, |ui| {
        ui.label(text);
        modal_buttons(ui, |ui| {
            if ui.button("Cancel").clicked() {
                answer = Some(false);
            }
            if ui.button(accept).clicked() {
                answer = Some(true);
            }
        });
    });
    if answer.is_none() && response.should_close() {
        answer = Some(false);
    }
    answer
}

/// What the prompt does once answered.
#[derive(Debug, Clone)]
pub(crate) enum Pending {
    /// The window's close (`Tool::hold_close`): Discard drops the session
    /// and asks the shell to close again, so the re-asked `hold_close`
    /// returns `false`.
    Close,
    /// The File menu's `Close save` (Ctrl+W): the save closes and the view
    /// draws its start again — the welcome panel.
    CloseSave,
    /// One of the File menu's opens: the save at the path opens.
    Open(PathBuf),
    /// The File menu's `Open…`: the file dialog follows the answer,
    /// in the folder given when there is one (a cancelled dialog leaves
    /// the open save as it is).
    OpenPicker(Option<PathBuf>),
}

/// The prompt's state on `SaveView`: the pending action and the table's
/// rows, computed once when the prompt opens, never per frame.
pub(crate) struct Prompt {
    /// What the prompt does once answered.
    pub(super) pending: Pending,
    /// The table's rows.
    rows: Vec<UnsavedRow>,
    /// The table's measured geometry, filled on its first frame.
    layout: TableLayout,
}

/// One row of the changes table: the item it changes (a player's or a
/// team's display name) and the field's terms, drawn as three columns,
/// the Item cell `<item> · <label>` as the draft joins them.
pub(super) struct ChangeRow {
    /// The player's or the team's display name.
    pub(super) item: String,
    /// The field's label, its two values and, for a team colour, their
    /// swatches.
    pub(super) term: Term,
}

/// One row of the unsaved changes (`rows`): its cells, and the change it
/// shows — what its X puts back, in the toolbar's list and the close
/// prompt alike, and what its item's click in the list switches to.
pub(super) struct UnsavedRow {
    /// The row's three cells.
    pub(super) cells: ChangeRow,
    /// The change the row shows, to put back alone (its X) and to switch
    /// to what it names (its item).
    pub(super) put_back: PutBack,
}

/// What `changes_table` drew: the row whose Item cell was clicked (a
/// `switch` table's), the header row's and each drawn row's Item cell on
/// its line (a column beside the table lines its own controls up with
/// them), and the lines drawn between the rows.
pub(super) struct TableResponse {
    /// The index of the row whose Item cell was clicked.
    pub(super) clicked: Option<usize>,
    /// Each drawn row's index and Item cell, in the rows' order.
    pub(super) items: Vec<(usize, egui::Rect)>,
    /// The header row's Item cell.
    pub(super) head: egui::Rect,
    /// The y of each drawn line between two rows, the header's included,
    /// centred in the row gap.
    pub(super) lines: Vec<f32>,
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
    // The popups the view owns would float over the prompt's layer.
    view.changes_list = None;
    view.tactics.close_popover();
    let Some(session) = view.session.as_ref() else {
        return;
    };
    if let Some(prompt) = view.prompt.as_mut() {
        prompt.pending = pending;
        return;
    }
    match rows(session) {
        Ok(rows) => {
            view.prompt = Some(Prompt {
                pending,
                rows,
                layout: TableLayout::default(),
            });
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
/// in the All teams list's order (a changed one the list does not show last by
/// id), a roster slot's `Shirt number` row under its player after his own
/// (the dot's rule: his dot covers his roster slot's number), then each changed team's
/// other rows in the open save's team order, first a `Player order` row per
/// group of its players that traded places (`places`). The item is the player's
/// display name (`display_name`, colour codes dropped) or the team's.
/// A team row's slot-valued texts name the baseline's roster and records
/// on the old side, the current's on the new: a drag moves records
/// between the roster's ids while the stored byte still names the slot.
pub(super) fn rows(session: &Session) -> anyhow::Result<Vec<UnsavedRow>> {
    let file = session.file();
    let version = session.version();
    let changes = session.unsaved_changes()?;
    let player_diffs: HashMap<u32, Vec<PlayerDiff>> = changes.players.into_iter().collect();
    let team_diffs: HashMap<u32, Vec<TeamDiff>> = changes.teams.into_iter().collect();
    // The All teams list runs to the thousands on a cup save: the id lookups
    // go through indexes built once, never `EditFile`'s linear scans
    // (`session`'s baseline lookups are already its own maps').
    let players: HashMap<u32, &PlayerEntry> = file
        .players()
        .iter()
        .map(|player| (player.id, player))
        .collect();
    let teams: HashMap<u32, &TeamEntry> = file.teams().iter().map(|team| (team.id, team)).collect();
    // The roster slots the All teams list shows (its order is the
    // players'): a `Shirt number` row of one goes under its player. The
    // table is not the list, so no category narrows it.
    let listed_rows = selection::rows(file, &TeamChoice::All, "", TeamCategory::All);
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
        if shown.insert(entry.player_id)
            && let Some(diffs) = player_diffs.get(&entry.player_id)
        {
            for diff in diffs {
                rows.push(UnsavedRow {
                    cells: ChangeRow {
                        item: named(),
                        term: terms::row_term(version, diff),
                    },
                    put_back: PutBack::Player {
                        id: entry.player_id,
                        diff: diff.clone(),
                    },
                });
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
            for (term, diffs) in terms::team_row_terms(
                version,
                team,
                baseline_team,
                &old_roster,
                &new_roster,
                &numbers,
            ) {
                // Under its player the row is his shirt number, as the
                // strip labels it: the slot's own words ("Player 04's
                // number") would name him twice.
                rows.push(UnsavedRow {
                    cells: ChangeRow {
                        item: named(),
                        term: Term {
                            label: info_strip::NUMBER_LABEL.to_owned(),
                            ..term
                        },
                    },
                    put_back: PutBack::Team {
                        id: entry.team_id,
                        diffs,
                    },
                });
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
            rows.push(UnsavedRow {
                cells: ChangeRow {
                    item: item.clone(),
                    term: terms::row_term(version, diff),
                },
                put_back: PutBack::Player {
                    id,
                    diff: diff.clone(),
                },
            });
        }
    }
    for team in file.teams() {
        for (_, group) in changes.moves.iter().filter(|(id, _)| *id == team.id) {
            rows.push(UnsavedRow {
                cells: ChangeRow {
                    item: display_name(&team.name),
                    term: Term {
                        label: PLAYER_ORDER.to_owned(),
                        old: places(group, |id| session.baseline_player(id)),
                        new: places(group, |id| players.get(&id).copied()),
                        swatches: None,
                    },
                },
                put_back: PutBack::Order(group.clone()),
            });
        }
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
        for (term, diffs) in terms::team_row_terms(
            version,
            team,
            baseline_team,
            &old_roster,
            &new_roster,
            &remaining,
        ) {
            rows.push(UnsavedRow {
                cells: ChangeRow {
                    item: display_name(&team.name),
                    term,
                },
                put_back: PutBack::Team { id: team.id, diffs },
            });
        }
    }
    Ok(rows)
}

/// The label of a group of moved players' row.
const PLAYER_ORDER: &str = "Player order";

/// One side of a `Player order` row (the draft's `places`): the group's
/// places in roster order, one per line, each its two-digit number and the
/// display name of the record `lookup` finds on its id (the baseline's on
/// the old side, the current file's on the new), so the two sides read
/// line by line and a long move makes the row taller, not wider.
fn places<'a>(group: &[MovedPlace], lookup: impl Fn(u32) -> Option<&'a PlayerEntry>) -> String {
    group
        .iter()
        .map(|place| {
            let name = lookup(place.id)
                .map_or_else(|| place.id.to_string(), |player| display_name(&player.name));
            format!("{:02} {name}", u32::from(place.slot) + 1)
        })
        .collect::<Vec<_>>()
        .join("\n")
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
                // The prompt words rows, never draws the pitch: it reads
                // no ratings.
                rated: None,
                fielded: &[],
            }
        })
        .collect()
}

/// The prompt while `view.prompt` holds one: the table and the three
/// answers. Cancel — the button, Esc or a click on the backdrop — closes
/// it and changes nothing; Discard goes on at once; Save writes first
/// and goes on only once the file is written (the running-game question
/// first when the save's own PES runs). A row's X puts that change back
/// alone, the prompt open a row shorter — or, the last row's, runs the
/// pending action as a clean save's prompt would.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>) {
    let Some(prompt) = view.prompt.as_mut() else {
        return;
    };
    let mut answer = None;
    let mut click = None;
    let id = egui::Id::unique("save_editor_unsaved");
    let response = modal(ui.ctx(), id, "Unsaved changes", |ui| {
        // The modal is as wide as its widest row, at most nine tenths of
        // the window's width: `set_max_width` caps the content (the frame's
        // margins and stroke make up the rest), and a wider row scrolls
        // sideways inside the table, no line of a cell wrapped.
        let chrome = modal_frame(ui.style()).total_margin().sum().x;
        ui.set_max_width((ui.ctx().content_rect().width() * 0.9 - chrome).max(0.0));
        click = table_with_xs(ui, &prompt.rows, &mut prompt.layout, None, 0.0);
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
    if let Some(TableClick::PutBack(index)) = click {
        put_back_row(view, index);
    }
}

/// The changes table's measured geometry: the three columns' widths and
/// every row's place, filled the first frame it draws and kept beside the
/// rows, which never change under it. A frame then lays out only the
/// header and the rows the scroll area shows, so a changes list thousands
/// of rows long scrolls at the speed of a dozen.
#[derive(Default)]
pub(super) struct TableLayout {
    /// `None` until the first frame measures (`measured`).
    measured: Option<Measured>,
}

/// What the first frame's measure found.
struct Measured {
    /// Each column's width (its widest cell's).
    widths: [f32; 3],
    /// The header row's three cells' sizes.
    heads: [egui::Vec2; 3],
    /// Each row's Item cell size, aligned with the rows.
    items: Vec<egui::Vec2>,
    /// Each row's two value cells' text sizes (a colour's swatch stands
    /// beside the text, not in it), aligned with the rows.
    values: Vec<[egui::Vec2; 2]>,
    /// Each row's top in content space, the header's first, the
    /// `TEXT_ROW_SPACING` gap between them counted in.
    tops: Vec<f32>,
    /// Each row's height, aligned with `tops`.
    heights: Vec<f32>,
}

impl TableLayout {
    /// The table's `Measured`, measuring it on first use: every cell's
    /// galley once, the same layout job `Label` runs (`Extend`, the
    /// ui's own text valign), so the sizes are the drawn cells'. Columns
    /// and rows take at least the interact size, as an interactive cell
    /// would.
    fn measured(&mut self, ui: &egui::Ui, rows: &[&ChangeRow]) -> &Measured {
        self.measured.get_or_insert_with(|| {
            let cell_min = ui.spacing().interact_size;
            let mut widths = [cell_min.x; 3];
            let mut heads = [egui::Vec2::ZERO; 3];
            let mut head_height = cell_min.y;
            for (column, text) in ["Item", "Old value", "New value"].into_iter().enumerate() {
                let size = galley_size(ui, egui::RichText::new(text));
                heads[column] = size;
                widths[column] = widths[column].max(size.x);
                head_height = head_height.max(size.y);
            }
            let mut items = Vec::with_capacity(rows.len());
            let mut values = Vec::with_capacity(rows.len());
            let mut heights = Vec::with_capacity(rows.len() + 1);
            heights.push(head_height);
            for ChangeRow { item, term } in rows {
                let item_size = galley_size(ui, format!("{item} · {}", term.label));
                let [old_swatch, new_swatch] = swatches(term);
                let old = galley_size(ui, egui::RichText::new(term.old.as_str()).monospace());
                let new = galley_size(ui, egui::RichText::new(term.new.as_str()).monospace());
                let old_whole = value_size(old, old_swatch);
                let new_whole = value_size(new, new_swatch);
                items.push(item_size);
                values.push([old, new]);
                widths[0] = widths[0].max(item_size.x);
                widths[1] = widths[1].max(old_whole.x);
                widths[2] = widths[2].max(new_whole.x);
                heights.push(
                    item_size
                        .y
                        .max(old_whole.y)
                        .max(new_whole.y)
                        .max(cell_min.y),
                );
            }
            let mut tops = Vec::with_capacity(heights.len());
            let mut top = 0.0;
            for &height in &heights {
                tops.push(top);
                top += height + TEXT_ROW_SPACING;
            }
            Measured {
                widths,
                heads,
                items,
                values,
                tops,
                heights,
            }
        })
    }
}

/// The row's two swatch colours, `None`s for a non-colour row.
fn swatches(term: &Term) -> [Option<egui::Color32>; 2] {
    match term.swatches {
        Some([old, new]) => [Some(old), Some(new)],
        None => [None, None],
    }
}

/// The size `text` lays out to as a table cell: one galley, never
/// wrapped, the way `cell`'s `Label` lays it out (the `Extend` wrap
/// mode's infinite measure, the ui's text valign).
fn galley_size(ui: &egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Vec2 {
    let mut job = std::sync::Arc::unwrap_or_clone(text.into().into_layout_job(
        ui.style(),
        egui::FontSelection::Default,
        ui.text_valign(),
    ));
    job.wrap.max_width = f32::INFINITY;
    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job)).size()
}

/// The changes table (the draft's `changesTable`), the prompt's, the
/// toolbar's changes list's and the patch panel's: the header row
/// `Item`, `Old value`, `New value` in the strong text colour, then one
/// row per `ChangeRow`, the Item cell `<item> · <label>` and the values
/// in monospace, a team colour's each after its swatch, no line wrapped.
/// Only the header and the rows `ui`'s clip shows are laid out (`layout`
/// measured every cell's size and place once), a row just past either
/// edge included so none pops in. A
/// thin line in the separators' stroke parts each row from the next, the
/// header's included, centred in the row gap across the table's width,
/// so a tall row's values stay with its item. A
/// value holding several lines (a `Player order` row's places) makes its
/// row that many lines high, the Item on the first (the draft's cells sit
/// on their baseline); every other row is one line. `switch` gives each
/// row's Item cell its hover text: `None` makes none a click target (the
/// prompt's items take no click), `Some(tips)` makes row `i`'s one where
/// `tips[i]` is `Some`.
pub(super) fn changes_table<'a>(
    ui: &mut egui::Ui,
    rows: impl IntoIterator<Item = &'a ChangeRow>,
    switch: Option<&[Option<&'static str>]>,
    layout: &mut TableLayout,
) -> TableResponse {
    let rows: Vec<&ChangeRow> = rows.into_iter().collect();
    let measured = layout.measured(ui, &rows);
    let separator = ui.visuals().widgets.noninteractive.bg_stroke;
    let column_gap = ui.spacing().item_spacing.x;
    let top_left = ui.cursor().min;
    // The table's whole extent claimed up front, so the scroll area
    // sizes the content although only the rows in view are laid out.
    let right = top_left.x
        + measured.widths[0]
        + measured.widths[1]
        + measured.widths[2]
        + 2.0 * column_gap;
    let bottom =
        top_left.y + measured.tops.last().unwrap_or(&0.0) + measured.heights.last().unwrap_or(&0.0);
    ui.advance_cursor_after_rect(egui::Rect::from_min_max(
        top_left,
        egui::pos2(right, bottom),
    ));
    let col_x = [
        top_left.x,
        top_left.x + measured.widths[0] + column_gap,
        top_left.x + measured.widths[0] + measured.widths[1] + 2.0 * column_gap,
    ];
    // The rows the clip shows, plus the one sliding into view at either
    // edge (a row's pitch: the interact height and the row gap).
    let overscan = ui.spacing().interact_size.y + TEXT_ROW_SPACING;
    let clip = ui.clip_rect();
    let mut head = egui::Rect::NOTHING;
    let mut clicked = None;
    let mut items = Vec::new();
    let mut lines = Vec::new();
    let mut draw_separator = |ui: &egui::Ui, after: usize| {
        // A line centred in the gap after entry `after`, the last entry's
        // gap bordering only the row it ends so none is drawn there.
        if after + 1 < measured.tops.len() {
            let y = top_left.y
                + measured.tops[after]
                + measured.heights[after]
                + 0.5 * TEXT_ROW_SPACING;
            if y >= clip.min.y - overscan && y <= clip.max.y + overscan {
                ui.painter().hline(top_left.x..=right, y, separator);
                lines.push(y);
            }
        }
    };
    for (column, text) in ["Item", "Old value", "New value"].into_iter().enumerate() {
        let size = measured.heads[column];
        let y = top_left.y + (measured.heights[0] - size.y) / 2.0;
        let rect = egui::Rect::from_min_size(egui::pos2(col_x[column], y), size);
        let response = put_cell(
            ui,
            rect,
            ("head", column),
            cell(egui::RichText::new(text).color(ui.visuals().strong_text_color())),
        );
        if column == 0 {
            head = response.rect;
        }
    }
    draw_separator(ui, 0);
    for (index, row) in rows.iter().enumerate() {
        let entry = index + 1;
        let row_top = top_left.y + measured.tops[entry];
        let row_height = measured.heights[entry];
        let row_bottom = row_top + row_height;
        if row_bottom >= clip.min.y - overscan && row_top <= clip.max.y + overscan {
            let size = measured.items[index];
            // A multi-line row's Item sits on the values' first line;
            // every other row's cells are centred in the row's height.
            let tall = row.term.old.contains('\n') || row.term.new.contains('\n');
            let y = if tall {
                row_top
            } else {
                row_top + (row_height - size.y) / 2.0
            };
            let rect = egui::Rect::from_min_size(egui::pos2(col_x[0], y), size);
            let tip = switch.and_then(|tips| tips.get(index)).copied().flatten();
            let mut item = cell(format!("{} · {}", row.item, row.term.label));
            if tip.is_some() {
                item = item.sense(egui::Sense::click());
            }
            let response = put_cell(ui, rect, ("item", index), item);
            items.push((index, response.rect));
            if let Some(tip) = tip
                && response.on_hover_text(tip).clicked()
            {
                clicked = Some(index);
            }
            let [old_swatch, new_swatch] = swatches(&row.term);
            for (column, cell_value) in [
                (row.term.old.as_str(), old_swatch),
                (row.term.new.as_str(), new_swatch),
            ]
            .into_iter()
            .enumerate()
            {
                draw_value(
                    ui,
                    col_x[1 + column],
                    row_top,
                    row_height,
                    cell_value,
                    measured.values[index][column],
                    ("value", index, column),
                );
            }
        }
        draw_separator(ui, entry);
    }
    TableResponse {
        clicked,
        items,
        head,
        lines,
    }
}

/// `widget` centred in `rect`, under an id `id_salt` gives the cell, so a
/// widget's id does not change as its row scrolls in and out of view.
fn put_cell(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id_salt: impl egui::AsIdSalt,
    widget: impl egui::Widget,
) -> egui::Response {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            ))
            .id_salt(id_salt),
        |ui| ui.add(widget),
    )
    .inner
}

/// A colour value's swatch side and its gap to the hex (the draft's
/// `.chswatch`).
const SWATCH: f32 = 12.0;
const SWATCH_GAP: f32 = 5.0;

/// A value cell's size: the text's, after its swatch when it has one.
fn value_size(text: egui::Vec2, swatch: Option<egui::Color32>) -> egui::Vec2 {
    match swatch {
        Some(_) => egui::vec2(SWATCH + SWATCH_GAP + text.x, text.y.max(SWATCH)),
        None => text,
    }
}

/// A value cell of the changes table drawn inside its column at `x`:
/// `value` in monospace, after its colour's swatch when it is a colour
/// (the draft's `.chswatch`): a 12-point square of the colour, its
/// corners the theme's radius, framed by the 1-point dark edge the
/// appearance palettes' swatches draw, so a colour near the panel's own
/// still shows its square, and 5 points clear of the hex. `size` is the
/// text's galley size as the first frame measured it.
fn draw_value(
    ui: &mut egui::Ui,
    x: f32,
    row_top: f32,
    row_height: f32,
    (value, swatch): (&str, Option<egui::Color32>),
    size: egui::Vec2,
    id_salt: impl egui::AsIdSalt,
) {
    let whole = value_size(size, swatch);
    let group =
        egui::Rect::from_min_size(egui::pos2(x, row_top + (row_height - whole.y) / 2.0), whole);
    let mut text_left = group.left();
    if let Some(colour) = swatch {
        let swatch_rect = egui::Rect::from_center_size(
            egui::pos2(group.left() + SWATCH / 2.0, group.center().y),
            egui::vec2(SWATCH, SWATCH),
        );
        let edge = studio_core::theme::palette(ui.visuals()).swatch_edge;
        ui.painter().rect(
            swatch_rect,
            ui.visuals().widgets.noninteractive.corner_radius,
            colour,
            egui::Stroke::new(1.0, edge),
            egui::StrokeKind::Inside,
        );
        text_left += SWATCH + SWATCH_GAP;
    }
    let text = egui::Rect::from_min_size(
        egui::pos2(text_left, group.top() + (whole.y - size.y) / 2.0),
        size,
    );
    put_cell(
        ui,
        text,
        id_salt,
        cell(egui::RichText::new(value).monospace()),
    );
}

/// A cell of the changes table: `text` on one line.
fn cell(text: impl Into<egui::WidgetText>) -> egui::Label {
    egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend)
}

/// The tooltip of a row's X.
const X_TOOLTIP: &str = "Undoes this change alone";

/// A click inside `table_with_xs`.
pub(super) enum TableClick {
    /// A row's X: puts that change back alone.
    PutBack(usize),
    /// A row's item: switches to what the row names (the toolbar's
    /// list's; the prompt's items take no click).
    Switch(usize),
}

/// The changes table with an X ending each row under an `Undo` head
/// (the draft's `changesTable`), the toolbar's changes list's and the
/// close prompt's alike, scrolling up and down past 240 points; the
/// click inside it this frame. The X's and their head stand in a column
/// right of the sideways scroll area, inside the one that scrolls up
/// and down, each on its row's first line: a wide row scrolls sideways
/// under the column, so neither the head nor an X leaves the view. A
/// table narrower than `min_width` (the list's floor; the prompt has
/// none) keeps the column at its right edge, and each separator line is
/// carried on under the column, so it runs the whole row. `tips` is
/// `changes_table`'s `switch`: the rows' item tooltips.
pub(super) fn table_with_xs(
    ui: &mut egui::Ui,
    rows: &[UnsavedRow],
    layout: &mut TableLayout,
    tips: Option<&[Option<&'static str>]>,
    min_width: f32,
) -> Option<TableClick> {
    let error = ui.visuals().error_fg_color;
    let separator = ui.visuals().widgets.noninteractive.bg_stroke;
    // The head, drawn as the table's other heads are (the strong text
    // colour); its width sets the column's.
    let galley =
        egui::WidgetText::from(egui::RichText::new("Undo").color(ui.visuals().strong_text_color()))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Body,
            );
    let x_width = ui.spacing().interact_size.y.max(galley.size().x);
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let left = ui.cursor().min.x;
                let spacing = ui.spacing().item_spacing.x;
                let table_width = (ui.available_width() - x_width - spacing).max(0.0);
                let table = egui::ScrollArea::horizontal()
                    .id_salt("save_editor_changes_list_sideways")
                    .max_width(table_width)
                    .show(ui, |ui| {
                        changes_table(ui, rows.iter().map(|row| &row.cells), tips, layout)
                    });
                let column = ui.cursor().min.x.max(left + min_width - x_width);
                // The head centred in the column, on the header row's
                // line (its galley centred in a rect of its own size).
                ui.put(
                    egui::Rect::from_center_size(
                        egui::pos2(column + 0.5 * x_width, table.inner.head.center().y),
                        galley.size(),
                    ),
                    egui::Label::new(egui::WidgetText::Galley(galley.clone())),
                );
                // Each separator line carried on under the column: drawn
                // from the row's left edge so the stretch under the table
                // (which the sideways scroll clips) meets the table's own.
                for y in &table.inner.lines {
                    ui.painter()
                        .hline(table.inner.head.left()..=column + x_width, *y, separator);
                }
                let mut click = table.inner.clicked.map(TableClick::Switch);
                // The X's salt runs from the first drawn row's index, so
                // a row's X keeps its id as it scrolls in and out of view.
                ui.skip_ahead_auto_ids(table.inner.items.first().map_or(0, |(index, _)| *index));
                for &(index, item) in &table.inner.items {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(column, item.top()),
                        egui::vec2(x_width, item.height()),
                    );
                    let x =
                        egui::Button::new(egui::RichText::new("\u{d7}").color(error)).frame(false);
                    if ui.put(rect, x).on_hover_text(X_TOOLTIP).clicked() {
                        click = Some(TableClick::PutBack(index));
                    }
                }
                click
            })
            .inner
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
            view.close_save();
            view.request_close = true;
        }
        Pending::CloseSave => view.close_save(),
        Pending::Open(path) => view.open(&path),
        Pending::OpenPicker(start) => {
            if let Some(path) = (view.open_dialog)(start.as_deref()) {
                view.open(&path);
            }
        }
    }
}

/// `row`'s put-back as one undo step inside `following_selection` — the
/// X's write, the toolbar's list's and the close prompt's alike:
/// `true` when it ran. Its error (the row stale under it) shows as the
/// refusal a put-back gets; `false` also with the session gone, which
/// no drawn table reaches.
pub(super) fn put_back(view: &mut SaveView, row: &PutBack) -> bool {
    match following_selection(view, |session| session.put_back(row)) {
        Some(Ok(())) => true,
        Some(Err(error)) => {
            view.refusal = Some(Refusal {
                title: "Cannot undo this change",
                message: format!("{error:#}"),
            });
            false
        }
        None => false,
    }
}

/// The X of the prompt's row `index` clicked: its change put back, then
/// the prompt open over the rows computed again, their layout measured
/// anew — or, nothing left to save or lose, `pending` run as a clean
/// save's prompt would answer, the prompt closed. A put-back error
/// keeps the question up, as a refused write does.
fn put_back_row(view: &mut SaveView, index: usize) {
    // The prompt comes out of the view for the call: `put_back` runs
    // inside `following_selection`, which borrows the view mutably.
    let Some(prompt) = view.prompt.take() else {
        return;
    };
    if !put_back(view, &prompt.rows[index].put_back) {
        view.prompt = Some(prompt);
        return;
    }
    let session = view.session.as_ref().expect("the put-back ran on it");
    if !session.is_dirty() {
        go_on(view, prompt.pending);
        return;
    }
    match rows(session) {
        Ok(rows) => {
            view.prompt = Some(Prompt {
                pending: prompt.pending,
                rows,
                layout: TableLayout::default(),
            });
        }
        Err(error) => {
            view.refusal = Some(Refusal {
                title: "Cannot compare the save",
                message: format!("{error:#}"),
            });
        }
    }
}

/// The running-game question `view.edit_mode_question` holds open
/// (SE-GUI-21's window rule): `<game> is running` says how to keep the
/// write, the game's Edit mode open or closed. Save writes — a
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
            "If the game's Edit mode is open, use its Data Management > Load after saving, \
             before you leave Edit mode. Leaving Edit mode after changing anything in it \
             writes the game's own copy over this save.",
        );
        modal_sentence_gap(ui);
        ui.label("If Edit mode is closed, the changes show when you open it.");
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
