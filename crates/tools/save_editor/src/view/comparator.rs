//! The comparator panel (the draft's `comparatorPanel`): an `egui::Window`
//! comparing the open save, edits included, against a picked second save of the same
//! version — each row picked → open in the views' terms (`terms::row_term`),
//! the rows grouped per team, a row's click selecting the player. The
//! comparison runs on a worker thread (two real PES 2021 saves differ in
//! 34,379 rows, about half a second), started when its key — the open
//! save's revision, the picked save's and the scope — moves on and the edit
//! has settled (`edit_settled`, the same condition the undo stack's seal
//! uses); until the reply lands the last result stays, marked `Comparing…`,
//! and a frame draws only the rows in view (`ScrollArea::show_rows`). The
//! team filter narrows the held list on the frame, so it starts no run;
//! Export writes the scope's `diff::report_lines` over the whole save, the
//! filter notwithstanding — the text `save-editor diff` prints.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use pes_savefile::ops::compare::{CompareError, DiffScope};
use pes_version::PesVersion;
use studio_core::Recorder;
use team_widgets::violations::{
    RULE_COL, RULE_GAP, SWITCH_TOOLTIP, SelectedPlayer, drow_columns, player_head, player_head_job,
    row_paint,
};

use crate::diff::{self, DiffReport, PlayerReport};
use crate::messages::version_mismatch_sentence;
use crate::session::Session;
use crate::session::journal;
use crate::session::selection::TeamChoice;
use crate::view::worker::{Pending, WorkerResult};
use crate::view::{
    Panel, Refusal, SaveView, aatf_check, edit_settled, player_list, refuse_save_being_read,
    surrender_focus, terms,
};

/// The panel's first width in points, its whole frame included (the
/// draft's `.panel`).
const PANEL_WIDTH: f32 = 620.0;

/// A field row's Old value column (the draft's `#cmp-rows .drow .o`): 200
/// points, its text cut with an ellipsis when longer, so every New value
/// starts at one place.
const OLD_COL: f32 = 200.0;

/// The list's first line whenever it holds a field row: the three columns'
/// head (`Line::Columns`).
static COLUMNS: Line = Line::Columns;

/// The pick's Open dialog (`rfd` in the window, a stub in the tests — a
/// native dialog cannot be driven headless; `SaveFileDialog`'s pattern).
pub(crate) type OpenSaveDialog = Box<dyn FnMut() -> Option<PathBuf>>;

/// A Save-file dialog: the suggested file name to the picked path
/// (`rfd` in the window, a stub in the tests — a native dialog cannot be
/// driven headless; the Ruleset editor's `SaveDialog` pattern). The
/// Export's.
pub(crate) type SaveFileDialog = Box<dyn FnMut(&str) -> Option<PathBuf>>;

/// The `rfd` Open dialog.
#[cfg(not(test))]
pub(crate) fn default_open_dialog() -> OpenSaveDialog {
    Box::new(|| rfd::FileDialog::new().pick_file())
}

/// `default_open_dialog` under `cargo test`: a test reaching a real file
/// dialog panics; tests stub `view.open_save_dialog`.
#[cfg(test)]
pub(crate) fn default_open_dialog() -> OpenSaveDialog {
    Box::new(|| panic!("a test reached a real file dialog: stub open_save_dialog"))
}

/// The `rfd` Save dialog for the Export: the suggested name comes from
/// the caller.
#[cfg(not(test))]
pub(crate) fn default_export_dialog() -> SaveFileDialog {
    Box::new(|name| {
        rfd::FileDialog::new()
            .add_filter("Text", &["txt"])
            .set_file_name(name)
            .save_file()
    })
}

/// `default_export_dialog` under `cargo test`: a test reaching a real
/// file dialog panics; tests stub `view.export_dialog`.
#[cfg(test)]
pub(crate) fn default_export_dialog() -> SaveFileDialog {
    Box::new(|_| panic!("a test reached a real file dialog: stub export_dialog"))
}

/// The panel's state on `SaveView` (`view.comparator`): the picked save,
/// the last pick's refusal, the scope, the team filter, the last export's
/// path and the comparison's worker state.
pub(super) struct Comparator {
    /// The picked second save (the report's `first`): its path and its
    /// session, opened through `Session::open_for_comparison` so a refusal
    /// is the toolbar's own (`<path>: cannot open the save`).
    pub(super) picked: Option<(PathBuf, Session)>,
    /// The last pick's refusal — a save of another version or one that did
    /// not open — drawn error-coloured under the pick row.
    refused: Option<String>,
    /// The Show row's scope (`None`: All).
    pub(super) scope: Option<DiffScope>,
    /// The Team combo's pick (`All`: every line). Narrows the drawn list
    /// alone: the comparison's key does not hold it, so a change starts
    /// no run.
    pub(super) filter_team: TeamChoice,
    /// The Export's written path, for the `Wrote <path>` line.
    pub(super) exported: Option<PathBuf>,
    /// The comparison: the last result with the key it ran for and the run
    /// in flight (`WorkerResult`'s cycle, the AATF checks' own).
    pub(super) comparison: WorkerResult<CompareKey, Result<Vec<Line>, CompareError>>,
}

/// What a comparison ran for: a change of the open save (`Session::
/// revision`), a new pick or a new scope recomputes it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CompareKey {
    /// `Session::revision` of the open save when the run was due.
    pub(super) revision: u64,
    /// The picked session's `Session::revision` — a re-pick of the same
    /// path is a fresh session, so the file's own changes recompute.
    pub(super) picked: u64,
    /// The scope the run's rows keep (`None`: All).
    pub(super) scope: Option<DiffScope>,
}

impl Comparator {
    /// Nothing picked.
    pub(super) fn new() -> Comparator {
        Comparator {
            picked: None,
            refused: None,
            scope: None,
            filter_team: TeamChoice::All,
            exported: None,
            comparison: WorkerResult::new(),
        }
    }

    /// Another save opened (`SaveView::open`): the pick stays — a member
    /// compares several saves against one reference — unless the new save
    /// is of another version, refused as any pick is, and the refusal
    /// recorded as the pick's own would be; the result and the comparison
    /// in flight go either way, and the team filter back to All (another
    /// save's teams are not this one's) with the export's line.
    pub(super) fn reset(&mut self, open_version: PesVersion, recorder: &Recorder) {
        self.comparison.reset();
        self.refused = None;
        self.filter_team = TeamChoice::All;
        self.exported = None;
        let picked_version = self.picked.as_ref().map(|(_, picked)| picked.version());
        if let Some(picked) = picked_version
            && picked != open_version
        {
            let path = self.picked.take().expect("picked above").0;
            let refused = mismatch_refusal(picked, open_version);
            journal::refused(&path, recorder, &refused);
            self.refused = Some(refused);
        }
    }
}

/// One flat line of the list, exactly one text row high so
/// `ScrollArea::show_rows` can draw only the rows in view.
pub(super) enum Line {
    /// A strong heading: `Team <id> <name>`, `No team`, or an only-in
    /// section's.
    Heading(String),
    /// A player's head: the lists' hyphenated `Player <id> - <number> -
    /// <name>` (`player_head_job` over `rostered`) when a roster holds
    /// him, `text` as one weak run when none does; `text` is the
    /// accessible label either way. `select` on every head whose player
    /// sits on an open-save roster (the picked-only ones have none).
    Head {
        /// The head's text.
        text: String,
        /// The parts the drawn head needs of a rostered player; `None`
        /// (no roster holds him) draws `text` alone.
        rostered: Option<Rostered>,
        /// The player a click selects.
        select: Option<SelectedPlayer>,
        /// His roster team in the save the head is read from (the team
        /// filter keeps the line for it; `None`: no roster holds him).
        team: Option<u32>,
    },
    /// A field's row in the views' terms (the draft's `#cmp-rows .drow`):
    /// the label, the picked save's value and the open save's in three
    /// columns; `None` select on a player no open roster holds (SE-GUI-13:
    /// listed and not clickable).
    Row {
        /// The field's label, the Item column.
        label: String,
        /// The picked save's value, the Old value column.
        old: String,
        /// The open save's value, the New value column.
        new: String,
        /// The player a click selects.
        select: Option<SelectedPlayer>,
        /// His open-save roster team, as the head's.
        team: Option<u32>,
    },
    /// The rows' head, `Item`, `Old value`, `New value` (the draft's
    /// `#cmp-rows .drow.head`). Never in a comparison's held lines:
    /// `shown` puts it first when the drawn list holds a row.
    Columns,
}

/// The window for one frame, while it is the open panel and a save is
/// open: the pick row, the last pick's refusal, the Show row, the
/// `Comparing…` mark and the list. The reply is polled before a new run is
/// decided on, so the frame that starts one draws `Comparing…` over the
/// previous result. A closed panel starts no run.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
        panel,
        comparator,
        session,
        team,
        filter,
        selected,
        refusal,
        open_save_dialog,
        export_dialog,
        focused,
        tactics,
        ..
    } = view;
    if *panel != Some(Panel::Comparator) {
        return;
    }
    let Some(session) = session.as_ref() else {
        return;
    };
    let file = session.file();
    let (_, end) = edit_settled(ui, *focused);
    // A run is due when the key moved on since the last result (a pick is
    // always of the open save's version: the pick and `reset` refuse the
    // rest).
    let comparing = if let Some((_, picked)) = &comparator.picked {
        let key = CompareKey {
            revision: session.revision(),
            picked: picked.revision(),
            scope: comparator.scope,
        };
        comparator
            .comparison
            .update(&key, true, end.settled(), |key| {
                spawn_compare(ui.ctx(), picked.file(), file, key)
            })
    } else {
        false
    };
    let mut open = true;
    panel_window(ui, "Comparator", "save_editor_comparator")
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            // One line: the label, the path truncated to the room the
            // buttons leave, the buttons at the row's right — a real
            // save's path once wrapped the buttons onto a line of their
            // own.
            ui.horizontal_top(|ui| {
                ui.weak("Second save ");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if let Some((picked_path, picked)) = &comparator.picked
                        && ui
                            .button("Export\u{2026}")
                            .on_hover_text(
                                "Write the differences of the shown scope as text, \
                                 covering every team",
                            )
                            .clicked()
                        && let Some(path) = export_dialog("diff.txt")
                    {
                        let picked = (picked_path.as_path(), picked);
                        match write_report(&path, session, picked, comparator.scope) {
                            Ok(()) => comparator.exported = Some(path),
                            Err(error) => {
                                *refusal = Some(Refusal {
                                    title: "Cannot export the differences",
                                    message: format!("{error:#}"),
                                });
                            }
                        }
                    }
                    if ui
                        .button("Pick\u{2026}")
                        .on_hover_text("The save the open one is compared against")
                        .clicked()
                    {
                        surrender_focus(ui);
                        if let Some(path) = open_save_dialog() {
                            comparator.exported = None;
                            match Session::open_for_comparison(&path, session.recorder().clone()) {
                                Ok(picked) if picked.version() == file.version() => {
                                    comparator.picked = Some((path, picked));
                                    comparator.refused = None;
                                }
                                Ok(picked) => {
                                    comparator.picked = None;
                                    comparator.comparison.reset();
                                    let refused =
                                        mismatch_refusal(picked.version(), file.version());
                                    journal::refused(&path, session.recorder(), &refused);
                                    comparator.refused = Some(refused);
                                }
                                Err(error) => {
                                    comparator.picked = None;
                                    comparator.comparison.reset();
                                    comparator.refused = Some(format!("{error:#}"));
                                }
                            }
                        }
                    }
                    match &comparator.picked {
                        Some((path, _)) => {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(path.display().to_string()).monospace(),
                                )
                                .truncate(),
                            );
                        }
                        None => {
                            ui.weak("(none)");
                        }
                    }
                });
            });
            if let Some(refused) = &comparator.refused {
                ui.colored_label(ui.visuals().error_fg_color, refused.as_str());
            }
            if comparator.picked.is_some() {
                ui.horizontal_wrapped(|ui| {
                    ui.weak("Show ");
                    for (scope, label) in [
                        (None, "All"),
                        (Some(DiffScope::Gameplay), "Gameplay"),
                        (Some(DiffScope::Aesthetics), "Aesthetics"),
                    ] {
                        if ui
                            .selectable_value(&mut comparator.scope, scope, label)
                            .clicked()
                        {
                            surrender_focus(ui);
                        }
                    }
                    ui.weak(" Team ");
                    player_list::team_selector(
                        ui,
                        &mut comparator.filter_team,
                        file,
                        "save_editor_comparator_team",
                    );
                });
            }
            if let Some(path) = &comparator.exported {
                ui.weak(format!("Wrote {}", path.display()));
            }
            // Above the list, so a member scrolled deep in the rows still
            // sees that they are the previous result's.
            if comparing {
                ui.weak("Comparing\u{2026}");
            }
            match &comparator.comparison.result {
                Some((_, Ok(lines))) if lines.is_empty() => {
                    ui.weak("No differences.");
                }
                Some((_, Ok(lines))) => {
                    let shown = shown(lines, comparator.filter_team);
                    if shown.is_empty() {
                        ui.weak("No differences in this team.");
                    } else {
                        let clicked = egui::ScrollArea::vertical()
                            .id_salt("save_editor_comparator_rows")
                            .auto_shrink([false, false])
                            .show_rows(
                                ui,
                                ui.text_style_height(&egui::TextStyle::Body),
                                shown.len(),
                                |ui, range| {
                                    let mut clicked = None;
                                    for i in range {
                                        if let Some(player) = line(ui, shown[i]) {
                                            clicked = Some(player);
                                        }
                                    }
                                    clicked
                                },
                            )
                            .inner;
                        if let Some(player) = clicked {
                            // A listed row's team is one of the open
                            // file's.
                            let team_tactics = file
                                .team(player.team_id)
                                .map(|entry| &entry.tactics)
                                .expect("a listed row's team is in the open file");
                            aatf_check::select_player(
                                player,
                                selected,
                                team,
                                filter,
                                team_tactics,
                                tactics,
                            );
                        }
                    }
                }
                Some((_, Err(error))) => {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        format!("The comparison failed: {error}"),
                    );
                }
                None => {
                    if comparator.picked.is_none() {
                        ui.weak("Pick a save of the same version to compare against.");
                    }
                }
            }
        });
    if !open {
        *panel = None;
    }
}

/// A floating panel's window (the Comparator's, the patch panel's, the team
/// file panel's), titled `title`, with `id` its own. It first opens where
/// the draft's `.panel` stands: its right edge 1 inside the window's, its
/// top 90 below, over the views' right columns rather than the player list
/// a member picks from while it is open. egui places a window by its
/// default once, then keeps where the member drags it.
pub(super) fn panel_window<'open>(ui: &egui::Ui, title: &str, id: &str) -> egui::Window<'open> {
    let screen = ui.ctx().content_rect();
    egui::Window::new(studio_core::theme::window_title_text(title))
        .id(egui::Id::unique(id))
        .default_pos(egui::pos2(
            screen.right() - 1.0 - PANEL_WIDTH,
            screen.top() + 90.0,
        ))
        .default_width(PANEL_WIDTH)
        .collapsible(false)
        .resizable(true)
}

/// Export's write: the `scope`'s difference report of `picked` (its path
/// and session) against `open`, over the whole save, the team filter
/// narrowing the screen alone, written to `path`. Both saves are being
/// read, so neither is taken as the destination. As `write_rosters_tsv`
/// writes its file, the error names the path. Each line ends in its own
/// `\n`, as the console writes them: an empty report is an empty file,
/// not "\n".
fn write_report(
    path: &Path,
    open: &Session,
    (picked_path, picked): (&Path, &Session),
    scope: Option<DiffScope>,
) -> anyhow::Result<()> {
    refuse_save_being_read(path, open.path(), "difference report", open.recorder())?;
    refuse_save_being_read(path, picked_path, "difference report", open.recorder())?;
    let text: String = diff::report_lines(picked.file(), open.file(), scope)
        .expect("two saves of one version compare")
        .iter()
        .map(|line| format!("{line}\n"))
        .collect();
    journal::write_and_record(open.recorder(), path, text.as_bytes())
}

/// The pick's refusal for a save of another version (the CLI's
/// `version_mismatch_sentence`, the picked first as the report has it).
fn mismatch_refusal(picked: PesVersion, open: PesVersion) -> String {
    version_mismatch_sentence("The picked save", picked, "the open save", open)
}

/// One comparison's worker: both `EditFile`s cloned for the thread — the
/// view keeps owning them and a worker may not hold a `&` across frames;
/// the open save's clone costs ~4 ms on the PES 21 fixture (debug build,
/// measured 2026-10-24), nothing against the ~600 ms run — then
/// `diff::build` and the line list off it, the key riding the reply so the
/// caller knows what landed.
fn spawn_compare(
    ctx: &egui::Context,
    picked: &EditFile,
    open: &EditFile,
    key: &CompareKey,
) -> Pending<CompareKey, Result<Vec<Line>, CompareError>> {
    let (picked, open) = (picked.clone(), open.clone());
    let (sender, replies) = crossbeam_channel::unbounded();
    let scope = key.scope;
    let reply_key = key.clone();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let result =
            diff::build(&picked, &open, scope).map(|report| lines(&report, &picked, &open));
        // A newer comparison can replace the receiver mid-flight; that
        // send's failure is the superseded thread ending, not an error.
        if sender.send((reply_key, result)).is_err() {
            log::debug!("the comparator's caller dropped a superseded comparison");
        }
        ctx.request_repaint();
    });
    Pending {
        key: key.clone(),
        replies,
    }
}

/// The parts a rostered player's head draws from (`player_head_job`).
pub(super) struct Rostered {
    /// The player id.
    id: u32,
    /// His shirt number on the roster the head is read from.
    number: u16,
    /// His name as the save stores it (colour codes kept).
    name: String,
}

/// A player's place on the first roster holding him.
#[derive(Clone, Copy)]
pub(super) struct Place {
    /// The team's index in its file's team order (the groups' order).
    index: usize,
    /// The team's id.
    team_id: u32,
    /// His roster slot.
    slot: u8,
    /// His shirt number there.
    number: u16,
}

impl Place {
    /// The player a click on his lines selects.
    pub(super) fn select(self) -> SelectedPlayer {
        SelectedPlayer {
            team_id: self.team_id,
            slot: self.slot,
            fielded: None,
        }
    }
}

/// The first team in `file`'s order whose roster holds each player: one
/// group per player, not one per roster — a player on a club and a
/// national team reads once.
pub(super) fn roster_of(file: &EditFile) -> HashMap<u32, Place> {
    let mut roster = HashMap::new();
    for (index, team) in file.teams().iter().enumerate() {
        for (slot, entry) in team.roster.iter().enumerate() {
            if entry.player_id != 0 {
                roster.entry(entry.player_id).or_insert(Place {
                    index,
                    team_id: team.id,
                    slot: u8::try_from(slot).expect("a roster slot"),
                    number: entry.number,
                });
            }
        }
    }
    roster
}

/// `report`'s flat line list: one group per team in the open save's team
/// order — a player's team is the first open-save roster holding him, his
/// group's players in `report`'s order — `No team` after the teams, then
/// the players only in one save under their headings.
fn lines(report: &DiffReport, picked: &EditFile, open: &EditFile) -> Vec<Line> {
    let open_roster = roster_of(open);
    let picked_roster = roster_of(picked);
    let version = open.version();
    // A stable sort on the team's index: one heading per team, `No team`
    // (no roster holds him) last.
    let mut players: Vec<&PlayerReport> = report.players.iter().collect();
    players.sort_by_key(|player| {
        open_roster
            .get(&player.id)
            .map_or(usize::MAX, |place| place.index)
    });
    let mut lines = Vec::new();
    let mut group = None;
    for player in players {
        let place = open_roster.get(&player.id).copied();
        let team = place.map(|place| place.team_id);
        if group != Some(team) {
            lines.push(Line::Heading(match team {
                Some(id) => format!(
                    "Team {id} {}",
                    display_name(aatf_check::team_name(open, id))
                ),
                None => "No team".to_string(),
            }));
            group = Some(team);
        }
        // A player's head clicks like his rows do (SE-GUI-13).
        let (text, rostered) = head(open, place, player.id);
        lines.push(Line::Head {
            text,
            rostered,
            select: place.map(Place::select),
            team,
        });
        for row in &player.rows {
            let term = terms::row_term(version, &row.diff);
            lines.push(Line::Row {
                label: term.label,
                old: term.old,
                new: term.new,
                select: place.map(Place::select),
                team,
            });
        }
    }
    for (entries, heading, file, rosters, clickable) in [
        (
            &report.only_in_first,
            "Only in the picked save",
            picked,
            &picked_roster,
            false,
        ),
        (
            &report.only_in_second,
            "Only in the open save",
            open,
            &open_roster,
            true,
        ),
    ] {
        if entries.is_empty() {
            continue;
        }
        lines.push(Line::Heading(heading.to_owned()));
        for entry in entries {
            let place = rosters.get(&entry.id).copied();
            let (text, rostered) = head(file, place, entry.id);
            lines.push(Line::Head {
                text,
                rostered,
                select: place.filter(|_| clickable).map(Place::select),
                team: place.map(|place| place.team_id),
            });
        }
    }
    lines
}

/// `id`'s head as `file` holds him: `player_head` over his roster place
/// with the drawn head's `Rostered`, `Player <id> <name>` alone when no
/// roster holds him.
fn head(file: &EditFile, place: Option<Place>, id: u32) -> (String, Option<Rostered>) {
    let name = file
        .player(id)
        .map(|player| player.name.as_str())
        .unwrap_or("?");
    match place {
        Some(place) => (
            player_head(id, place.number, name),
            Some(Rostered {
                id,
                number: place.number,
                name: name.to_owned(),
            }),
        ),
        None => (format!("Player {id} {}", display_name(name)), None),
    }
}

/// `lines` narrowed to `team` (All: every line): the heads and rows whose
/// roster team is the picked one kept, a heading pushed before its first
/// kept line — a group with nothing kept (the `No team` one, an only-in
/// section with no player of the team's roster) loses its heading too —
/// and the columns' head first when a row is kept. Runs on the frame, on
/// the held result: a filter change starts no comparison.
fn shown(lines: &[Line], team: TeamChoice) -> Vec<&Line> {
    let mut shown = match team {
        TeamChoice::All => lines.iter().collect(),
        TeamChoice::Team(team) => {
            let mut shown = Vec::new();
            let mut heading = None;
            for line in lines {
                match line {
                    Line::Heading(_) => heading = Some(line),
                    Line::Head { team: kept, .. } | Line::Row { team: kept, .. } => {
                        if *kept == Some(team) {
                            if let Some(heading) = heading.take() {
                                shown.push(heading);
                            }
                            shown.push(line);
                        }
                    }
                    Line::Columns => {}
                }
            }
            shown
        }
    };
    if shown.iter().any(|line| matches!(line, Line::Row { .. })) {
        shown.insert(0, &COLUMNS);
    }
    shown
}

/// One line of the list, one text row high (the `violations` list's row
/// pattern, its `row_paint` tail shared): a heading in the strong color,
/// a head in the lists' hyphenated style (one weak run when no roster
/// holds the player), a row the draft's `#cmp-rows .drow` three columns
/// under their head; a row or a head with `select` is a click target
/// reporting the player.
fn line(ui: &mut egui::Ui, line: &Line) -> Option<SelectedPlayer> {
    let visuals = ui.visuals();
    match line {
        Line::Heading(text) => text_line(ui, text, visuals.strong_text_color(), None),
        Line::Head {
            text,
            rostered,
            select,
            ..
        } => head_line(ui, text, rostered.as_ref(), *select),
        Line::Row {
            label,
            old,
            new,
            select,
            ..
        } => field_row(ui, label, old, new, *select),
        Line::Columns => {
            columns_head(ui);
            None
        }
    }
}

/// The x offsets from a field row's left edge of its three columns, Item,
/// Old value and New value: `drow_columns`' two, the label's column
/// `RULE_COL` wide, then the New value `RULE_GAP` past the Old value's
/// `OLD_COL`.
fn column_offsets(width: f32) -> [f32; 3] {
    let (label_x, old_x, _) = drow_columns(width, RULE_COL);
    [label_x, old_x, old_x + OLD_COL + RULE_GAP]
}

/// The rows' head (the draft's `#cmp-rows .drow.head`): `Item`, `Old
/// value` and `New value` in the weak colour at the field rows' three
/// places, as the unsaved changes list heads its own. It names columns and
/// selects nothing, so it takes no click and draws no hover outline.
fn columns_head(ui: &mut egui::Ui) {
    let weak = ui.visuals().weak_text_color();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let offsets = column_offsets(ui.available_width());
    let galleys = ["Item", "Old value", "New value"].map(|title| {
        ui.painter()
            .layout_no_wrap(title.to_owned(), font.clone(), weak)
    });
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), galleys[0].size().y),
        egui::Sense::hover(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Label,
            ui.is_enabled(),
            "Item Old value New value",
        )
    });
    if ui.is_rect_visible(rect) {
        for (x, galley) in offsets.into_iter().zip(galleys) {
            ui.painter()
                .galley(rect.min + egui::vec2(x, 0.0), galley, weak);
        }
    }
}

/// A player's head line: the lists' hyphenated head (`player_head_job`)
/// for a rostered player, `text` as one weak run for a player no roster
/// holds; `text` is the row's accessible label either way.
fn head_line(
    ui: &mut egui::Ui,
    text: &str,
    rostered: Option<&Rostered>,
    select: Option<SelectedPlayer>,
) -> Option<SelectedPlayer> {
    let Some(player) = rostered else {
        return text_line(ui, text, ui.visuals().weak_text_color(), select);
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let job = player_head_job(player.id, player.number, &player.name, &font, ui.visuals());
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    galley_line(ui, galley, ui.visuals().text_color(), text, select)
}

/// A heading's or a head's line: `text` in `color` at the row's left edge.
fn text_line(
    ui: &mut egui::Ui,
    text: &str,
    color: egui::Color32,
    select: Option<SelectedPlayer>,
) -> Option<SelectedPlayer> {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    galley_line(ui, galley, color, text, select)
}

/// `galley` at the row's left edge (a heading's or a head's), `label` the
/// row's accessible label, `select` making the row a click target.
fn galley_line(
    ui: &mut egui::Ui,
    galley: std::sync::Arc<egui::Galley>,
    fallback: egui::Color32,
    label: &str,
    select: Option<SelectedPlayer>,
) -> Option<SelectedPlayer> {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), galley.size().y),
        if select.is_some() {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    row_paint(
        ui,
        response,
        &[egui::Shape::galley(rect.min, galley, fallback)],
        label,
        select,
        select.is_some().then_some(SWITCH_TOOLTIP),
    )
}

/// `text` on one line in `color`, cut with an ellipsis past `width`
/// points.
fn cut_galley(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat::simple(font, color),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    ui.fonts_mut(|f| f.layout_job(job))
}

/// A field's row (the draft's `#cmp-rows .drow`) at `column_offsets`'
/// three places: `label` in `RULE_COL` points of the plain text color, one
/// line cut with an ellipsis as the AATF rows' names are, then `old` cut
/// the same way at `OLD_COL` and `new` at the rest of the row's width,
/// both in the strong color. `<label> <old> → <new>` is the AccessKit
/// label, one name a screen reader reads; a row that cut any of the three
/// hovers that name too, above the switch hint on a row that selects.
fn field_row(
    ui: &mut egui::Ui,
    label: &str,
    old: &str,
    new: &str,
    select: Option<SelectedPlayer>,
) -> Option<SelectedPlayer> {
    let (text_color, strong_color) = {
        let visuals = ui.visuals();
        (visuals.text_color(), visuals.strong_text_color())
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let [label_x, old_x, new_x] = column_offsets(ui.available_width());
    let label_galley = cut_galley(ui, label, font.clone(), text_color, RULE_COL);
    let old_galley = cut_galley(ui, old, font.clone(), strong_color, OLD_COL);
    let new_galley = cut_galley(
        ui,
        new,
        font,
        strong_color,
        (ui.available_width() - new_x).max(0.0),
    );
    let height = label_galley
        .size()
        .y
        .max(old_galley.size().y)
        .max(new_galley.size().y);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        if select.is_some() {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let name = format!("{label} {old} \u{2192} {new}");
    let hover = if label_galley.elided || old_galley.elided || new_galley.elided {
        Some(match select {
            Some(_) => format!("{name}\n{SWITCH_TOOLTIP}"),
            None => name.clone(),
        })
    } else {
        select.map(|_| SWITCH_TOOLTIP.to_owned())
    };
    row_paint(
        ui,
        response,
        &[
            egui::Shape::galley(
                rect.min + egui::vec2(label_x, 0.0),
                label_galley,
                text_color,
            ),
            egui::Shape::galley(rect.min + egui::vec2(old_x, 0.0), old_galley, strong_color),
            egui::Shape::galley(rect.min + egui::vec2(new_x, 0.0), new_galley, strong_color),
        ],
        &name,
        select,
        hover.as_deref(),
    )
}
