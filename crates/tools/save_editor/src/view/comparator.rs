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
use std::path::PathBuf;

use anyhow::Context;
use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use pes_savefile::ops::compare::{CompareError, DiffScope};
use pes_version::PesVersion;
use team_widgets::violations::{SelectedPlayer, player_head, row_paint};

use crate::diff::{self, DiffReport, PlayerReport};
use crate::messages::{Side, version_mismatch_sentence};
use crate::session::Session;
use crate::session::selection::TeamChoice;
use crate::view::worker::{Pending, WorkerResult};
use crate::view::{
    Refusal, SaveView, aatf_check, edit_settled, player_list, surrender_focus, terms,
};

/// The panel's first width in points, its whole frame included (the
/// draft's `.panel`).
const PANEL_WIDTH: f32 = 620.0;

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

/// The panel's state on `SaveView` (`view.comparator`): open or not, the
/// picked save, the last pick's refusal, the scope, the team filter, the
/// last export's path and the comparison's worker state.
pub(super) struct Comparator {
    /// Whether the window is drawn (the Panels menu's `Comparator…` item).
    pub(super) open: bool,
    /// The picked second save (the report's `first`): its path and its
    /// session, opened through `Session::open` so a refusal is the
    /// toolbar's own (`<path>: cannot open the save`).
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
    /// The panel closed, nothing picked.
    pub(super) fn new() -> Comparator {
        Comparator {
            open: false,
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
    /// is of another version, refused as any pick is; the result and the
    /// comparison in flight go either way, and the team filter back to
    /// All (another save's teams are not this one's) with the export's
    /// line.
    pub(super) fn reset(&mut self, open_version: PesVersion) {
        self.comparison.reset();
        self.refused = None;
        self.filter_team = TeamChoice::All;
        self.exported = None;
        let picked_version = self.picked.as_ref().map(|(_, picked)| picked.version());
        if let Some(picked) = picked_version
            && picked != open_version
        {
            self.picked = None;
            self.refused = Some(mismatch_refusal(picked, open_version));
        }
    }
}

/// One flat line of the list, exactly one text row high so
/// `ScrollArea::show_rows` can draw only the rows in view.
pub(super) enum Line {
    /// A strong heading: `Team <id> <name>`, `No team`, or an only-in
    /// section's.
    Heading(String),
    /// A player's head (`violations`' `Player <id>  <number> <name>`
    /// format), weak, his stripped mark appended; `select` on every head
    /// whose player sits on an open-save roster (the picked-only ones have
    /// none).
    Head {
        /// The head's text.
        text: String,
        /// The player a click selects.
        select: Option<SelectedPlayer>,
        /// His roster team in the save the head is read from (the team
        /// filter keeps the line for it; `None`: no roster holds him).
        team: Option<u32>,
    },
    /// A field's row in the views' terms: `<label> <old> → <new>`; `None`
    /// select on a player no open roster holds (SE-GUI-13: listed and not
    /// clickable).
    Row {
        /// The row's text.
        text: String,
        /// The player a click selects.
        select: Option<SelectedPlayer>,
        /// His open-save roster team, as the head's.
        team: Option<u32>,
    },
}

/// The window for one frame, while `view.comparator.open` and a save is
/// open: the pick row, the last pick's refusal, the Show row, the
/// `Comparing…` mark and the list. The reply is polled before a new run is
/// decided on, so the frame that starts one draws `Comparing…` over the
/// previous result. A closed panel starts no run.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
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
    if !comparator.open {
        return;
    }
    let Some(session) = session.as_ref() else {
        return;
    };
    let file = session.file();
    let (_, settled) = edit_settled(ui, *focused);
    // A run is due when the key moved on since the last result (a pick is
    // always of the open save's version: the pick and `reset` refuse the
    // rest).
    let comparing = if let Some((_, picked)) = &comparator.picked {
        let key = CompareKey {
            revision: session.revision(),
            picked: picked.revision(),
            scope: comparator.scope,
        };
        comparator.comparison.update(&key, true, settled, |key| {
            spawn_compare(ui.ctx(), picked.file(), file, key)
        })
    } else {
        false
    };
    let mut open = comparator.open;
    // The panel first opens where the draft's `.panel` stands: its right
    // edge 1 inside the window's, its top 90 below, over the views' right
    // columns rather than the player list a member picks from while it is
    // open. egui places a window by its default once, then keeps where
    // the member drags it.
    let screen = ui.ctx().content_rect();
    egui::Window::new(studio_core::theme::window_title_text("Comparator"))
        .id(egui::Id::unique("save_editor_comparator"))
        .default_pos(egui::pos2(
            screen.right() - 1.0 - PANEL_WIDTH,
            screen.top() + 90.0,
        ))
        .default_width(PANEL_WIDTH)
        .collapsible(false)
        .resizable(true)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            // One line: the label, the path truncated to the room the
            // buttons leave, the buttons at the row's right — a real
            // save's path once wrapped the buttons onto a line of their
            // own.
            ui.horizontal_top(|ui| {
                ui.weak("Second save ");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if let Some((_, picked)) = &comparator.picked
                        && ui
                            .button("Export\u{2026}")
                            .on_hover_text(
                                "Write the differences of the shown scope as text, \
                                 covering every team",
                            )
                            .clicked()
                        && let Some(path) = export_dialog("diff.txt")
                    {
                        // As `write_rosters_tsv` writes its file: the error
                        // names the path. The scope's report over the whole
                        // save, the team filter narrowing the screen alone.
                        // Each line ends in its own `\n`, as the console writes
                        // them: an empty report is an empty file, not "\n".
                        match diff::report_lines(picked.file(), file, comparator.scope)
                            .map(|lines| {
                                lines
                                    .iter()
                                    .map(|line| format!("{line}\n"))
                                    .collect::<String>()
                            })
                            .map_err(anyhow::Error::from)
                            .and_then(|text| {
                                std::fs::write(&path, text)
                                    .with_context(|| path.display().to_string())
                            }) {
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
                            match Session::open(&path) {
                                Ok(picked) if picked.version() == file.version() => {
                                    comparator.picked = Some((path, picked));
                                    comparator.refused = None;
                                }
                                Ok(picked) => {
                                    comparator.picked = None;
                                    comparator.comparison.reset();
                                    comparator.refused =
                                        Some(mismatch_refusal(picked.version(), file.version()));
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
                    ui.selectable_value(&mut comparator.scope, None, "All");
                    ui.selectable_value(
                        &mut comparator.scope,
                        Some(DiffScope::Gameplay),
                        "Gameplay",
                    );
                    ui.selectable_value(
                        &mut comparator.scope,
                        Some(DiffScope::Aesthetics),
                        "Aesthetics",
                    );
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
    comparator.open = open;
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

/// A player's place on the first roster holding him.
#[derive(Clone, Copy)]
struct Place {
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
    fn select(self) -> SelectedPlayer {
        SelectedPlayer {
            team_id: self.team_id,
            slot: self.slot,
            fielded: None,
        }
    }
}

/// The first team in `file`'s order whose roster holds each player (the
/// `strip_again` precedent: one group per player, not one per roster — a
/// player on a club and a national team reads once).
fn roster_of(file: &EditFile) -> HashMap<u32, Place> {
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
                Some(id) => format!("Team {id} {}", aatf_check::team_name(open, id)),
                None => "No team".to_string(),
            }));
            group = Some(team);
        }
        // A player's head is his only line when he has no row in scope (a
        // stripped mark alone), so it clicks like a row does (SE-GUI-13).
        lines.push(Line::Head {
            text: head(open, place, player.id, player.stripped),
            select: place.map(Place::select),
            team,
        });
        for row in &player.rows {
            let term = terms::row_term(version, &row.diff);
            lines.push(Line::Row {
                text: format!("{} {} \u{2192} {}", term.label, term.old, term.new),
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
            lines.push(Line::Head {
                text: head(file, place, entry.id, entry.stripped),
                select: place.filter(|_| clickable).map(Place::select),
                team: place.map(|place| place.team_id),
            });
        }
    }
    lines
}

/// `id`'s head as `file` holds him: `player_head` over his roster place,
/// `Player <id> <name>` when no roster holds him, the stripped mark
/// appended when the report says he is stripped.
fn head(file: &EditFile, place: Option<Place>, id: u32, stripped: Option<Side>) -> String {
    let name = file
        .player(id)
        .map(|player| player.name.as_str())
        .unwrap_or("?");
    let mut head = match place {
        Some(place) => player_head(id, place.number, name),
        None => format!("Player {id} {}", display_name(name)),
    };
    if let Some(mark) = stripped.map(mark) {
        head.push_str(", ");
        head.push_str(mark);
    }
    head
}

/// `lines` narrowed to `team` (All: every line): the heads and rows whose
/// roster team is the picked one kept, a heading pushed before its first
/// kept line — a group with nothing kept (the `No team` one, an only-in
/// section with no player of the team's roster) loses its heading too.
/// Runs on the frame, on the held result: a filter change starts no
/// comparison.
fn shown(lines: &[Line], team: TeamChoice) -> Vec<&Line> {
    let TeamChoice::Team(team) = team else {
        return lines.iter().collect();
    };
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
        }
    }
    shown
}

/// The stripped mark for `side` (the report's `first` is the picked save,
/// `second` the open one).
fn mark(side: Side) -> &'static str {
    match side {
        Side::First => "stripped in the picked save",
        Side::Second => "stripped in the open save",
        Side::Both => "stripped in both",
    }
}

/// One line of the list, one text row high (the `violations` list's row
/// pattern, its `row_paint` tail shared): a heading in the strong color,
/// a head in the weak, a row in the plain; a row or a head with `select`
/// is a click target reporting the player.
fn line(ui: &mut egui::Ui, line: &Line) -> Option<SelectedPlayer> {
    let visuals = ui.visuals();
    let (text, color, select) = match line {
        Line::Heading(text) => (text, visuals.strong_text_color(), None),
        Line::Head { text, select, .. } => (text, visuals.weak_text_color(), *select),
        Line::Row { text, select, .. } => (text, visuals.text_color(), *select),
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(text.clone(), font, color);
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
        &[egui::Shape::galley(rect.min, galley, color)],
        text,
        select,
        select.is_some().then_some("Select this player"),
    )
}
