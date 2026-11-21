//! The player view's three columns (the draft's `playerView`/`.pcols`):
//! every panel a `Frame::group` as the Appearance screen's groups are,
//! the frames spanning the pane as theirs do — General info
//! (`info_strip::general`), Main stats (`info_strip::main_stats`,
//! Captain in its title's row) and Playable positions in the
//! first column, its width what the fixed columns leave; Skills with the
//! Cards used badge in its header row, the ruleset's apart cards (its
//! `tricks` group, else the skills its `free_cards` effects name) and COM
//! playing styles in the second, its width its cards'; the 25 ability
//! stats in the third, its width theirs — `COLUMN_GAP` apart, the first
//! frames' tops where the Appearance screen's first groups' stand, the
//! columns ending on one line with each last frame running down to the
//! tallest's bottom. Star and playing attitude sit on the Appearance
//! screen, in its Presentation group
//! (`appearance::fields::PRESENTATION_STATS`); the quick actions sit under
//! the player list (`quick_actions`).
//! Every control edits a copy of the player's entry and, when it changes,
//! pushes it with `Session::edit_player` under the control's `egui::Id`
//! value, as `info_strip.rs` does; edit flags are not touched. A field
//! the save's version does not store is drawn in its place disabled with
//! a `PES NN+` tooltip naming the first version that has it.

use std::ops::RangeInclusive;
use std::sync::Arc;

use aatf::TeamView;
use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::{
    self, ABILITY_STATS, FOOT, PLAYABLE_POSITIONS, POSITIONS, stat_name, title_case,
};
use pes_savefile::model::{PlayerEntry, PlayerPositions, PlayerSkills, PlayerStats};
use pes_savefile::schema::fields::{ABILITY_FIELDS, PlayerField};
use pes_savefile::schema::first_version_with;
use studio_core::widgets::{
    FIELD_SPACING, add_enabled, number_arrows, number_box, number_box_look, number_wheel_steps,
    resting_edge, step_number,
};
use team_widgets::cards;
use team_widgets::pitch::POSITION_ROWS;
use team_widgets::violations::Wording;

use crate::session::Session;
use crate::view::group_frame::{GROUP_GAP, stretch_to};
use crate::view::worker::{Pending, WorkerResult};
use crate::view::{
    ActiveRules, BadgeKey, OwnBadge, Refusal, SaveView, edit_player, edit_team, info_strip,
    live_marks, selected_player,
};

/// The Profile grid's registered-position combo.
pub(super) const REGISTERED_POSITION_LABEL: &str = "Registered position";
/// The Profile grid's playing-style picker.
pub(super) const PLAYING_STYLE_LABEL: &str = "Playing style";
/// The Profile grid's stronger-foot combo.
pub(super) const STRONGER_FOOT_LABEL: &str = "Stronger foot";
/// The Profile grid's stronger-hand combo.
pub(super) const STRONGER_HAND_LABEL: &str = "Stronger hand";

/// The playable positions' ratings, `(label, stored)` pairs: a stored
/// value past the table selects none of the three.
pub(super) const RATING_LABELS: [(&str, u8); 3] = [("C", 0), ("B", 1), ("A", 2)];

/// The playable positions' group heading; a diff row's own label starts
/// from it (`terms::row_term`).
pub(super) const PLAYABLE_POSITIONS_LABEL: &str = "Playable positions";

/// The layout the Playable positions grid draws in (the draft's
/// `POS_LAYOUT`): the toggle under the panel's title picks one of three,
/// kept as the `positions_layout` setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionsLayout {
    /// The pitch's five rows, each centred in the panel (the draft's
    /// `#pos-grid`).
    Pitch,
    /// The same rows starting at one left edge, the block the widest row
    /// makes centred in the panel (`#pos-grid.left`).
    Left,
    /// One position a row, front to back (`POSITION_LIST`), its label
    /// left of its C B A, the column centred (`#pos-grid.list`).
    List,
}

impl PositionsLayout {
    /// The layouts in the toggle's order.
    pub(crate) const CHOICES: [PositionsLayout; 3] = [
        PositionsLayout::Pitch,
        PositionsLayout::Left,
        PositionsLayout::List,
    ];

    /// The segment's text.
    pub(crate) fn label(self) -> &'static str {
        match self {
            PositionsLayout::Pitch => "Pitch",
            PositionsLayout::Left => "Left",
            PositionsLayout::List => "List",
        }
    }

    /// The `positions_layout` setting's spelling of the layout.
    pub(crate) fn setting(self) -> &'static str {
        match self {
            PositionsLayout::Pitch => "pitch",
            PositionsLayout::Left => "left",
            PositionsLayout::List => "list",
        }
    }

    /// The layout `setting` spells; an unknown spelling reads as `Pitch`.
    pub(crate) fn from_setting(setting: &str) -> PositionsLayout {
        match setting {
            "left" => PositionsLayout::Left,
            "list" => PositionsLayout::List,
            _ => PositionsLayout::Pitch,
        }
    }
}

/// The line a tier check's failure draws in the badge's place: it opens
/// the error in the refusal dialog (the quick actions' line's shape).
const TIER_CHECK_FAILED_LINE: &str = "Tier check failed: click to see why.";

/// Between two panels of one column (the draft's `.pcol` `gap`).
const PANEL_GAP: f32 = 8.0;
/// Between two columns (the draft's `#player-view` grid `gap`).
const COLUMN_GAP: f32 = 8.0;
/// The gap a panel's title stands above the first thing drawn under it —
/// a field's frame, a checkbox's painted square or a label row's text:
/// the team view's titles' spacing, so the two views read alike. The
/// Appearance screen's groups leave the same under their titles.
pub(super) const TITLE_GAP: f32 = 6.0;
/// The card columns' own margin under a card panel's title (the draft's
/// `.skcols` `margin-top` is 1.5, a plain content block's ~1): a card
/// panel's first checkbox stands half a point lower than the same
/// `TITLE_GAP` puts a field panel's first row.
const CARD_COLS_MARGIN: f32 = 0.5;
/// The padding of the ability stats' cells, either side of a label and
/// of a box (the draft's `stats-grid` `td`), and how far every panel's and
/// group's content stands in from its title (`inset`), so a title heads
/// its body.
pub(super) const CELL_PADDING: i8 = 6;
/// The padding of the ability stats' cells under a box (the draft's
/// `stats-grid` `td`), half of `FIELD_SPACING`.
const CELL_BOTTOM_PADDING: i8 = 3;

/// The first column's floor, which the column never goes under: General
/// info's nominal width and its inset.
fn first_column_width() -> f32 {
    info_strip::GENERAL_WIDTH + f32::from(CELL_PADDING)
}

/// The egui memory id `show` writes the two fixed columns' measured
/// widths to each pass — Skills' then Ability stats', clamped to their
/// floors (a pass that measures different ones is discarded). General
/// info's flexible width is not stored: it is already right in the pass
/// that computes it.
pub(super) const COLUMN_WIDTHS_ID: &str = "save_editor player column widths";

/// The egui memory id `show` writes the positions grid's extra room to
/// each pass — the tallest column's bottom less the first column's own
/// bottom measured with the grid at its rows' height (a pass that
/// measures a different one is discarded). The room is known only after
/// the columns are laid out, so the grid gets it a pass late, as the
/// fixed columns' widths are.
const GRID_ROOM_ID: &str = "save_editor positions grid room";

/// The Skills column's narrowest width, and the first pass's estimate:
/// the card columns' 330 with their 6-point inset inside the frame's
/// own. A grid's discarded sizing pass measures narrower and must not
/// hand the first column the room.
const SECOND_COLUMN_MIN: f32 = 350.0;
/// The Ability stats column's narrowest width, and the first pass's
/// estimate: the stats grid's 177.4 inside the frame.
const THIRD_COLUMN_MIN: f32 = 191.4;

/// The columns for the selected player, under `info_strip`'s title: the
/// three columns side by side spanning the pane — Skills' and Ability
/// stats' the width their content needs, General info's what they leave —
/// each a `column` of framed panels `PANEL_GAP` apart and `GROUP_GAP`
/// under the title's line, `COLUMN_GAP` between columns.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some((team_id, slot, player_id)) = selected_player(view) else {
        return;
    };
    let (version, positions, stats, mut skills, captain) = {
        let Some(session) = view.session.as_ref() else {
            return;
        };
        let file = session.file();
        let player = file.player(player_id).expect("the selected player");
        let team = file.team(team_id).expect("the selected team");
        (
            file.version(),
            player.positions.clone(),
            player.stats.clone(),
            player.skills.clone(),
            team.tactics.set_pieces.captain == slot,
        )
    };
    // Every widget id under the content is the player's within this open: a
    // DragValue's in-progress edit, a combo's popup and the pickers' state
    // never cross players — nor saves, since egui keys a TextEdit's undo
    // history by widget id and the same player id of the save opened next
    // would otherwise inherit the last save's (and the undo merge keys
    // differ per player, which is right).
    let SaveView {
        session,
        rules,
        cards: badge,
        generation,
        refusal,
        live,
        live_aatf,
        live_suggestions,
        positions_layout,
        focused,
        ..
    } = view;
    // Live AATF's held check while it is the selected player's team's:
    // the field crosses below read it, worded under the active rules and
    // the open save's stat names.
    let held = rules.as_ref().and_then(|active| {
        live_marks::held_violations(*live_aatf, *live_suggestions, &live.check, &active.key)
            .filter(|(id, _)| *id == team_id)
            .map(|(_, violations)| (violations, active))
    });
    let marks = held.as_ref().map(|(violations, active)| live_marks::Marks {
        violations,
        slot,
        team_id,
        wording: Wording {
            ruleset: active.rules.ruleset(),
            version,
        },
    });
    let Some(session) = session.as_mut() else {
        return;
    };
    let generation = *generation;
    let mut skills_changed = false;
    ui.push_id(("player", generation, player_id), |ui| {
        let spacing = ui.spacing().item_spacing;
        // The fixed columns' widths are the last pass's measured ones
        // (the constants when none is stored): the ability stats' grid
        // is new on a player's first frame and sizes itself in a
        // discarded pass.
        let measured: Option<[f32; 2]> = ui
            .ctx()
            .data(|data| data.get_temp(egui::Id::unique(COLUMN_WIDTHS_ID)));
        let saved = measured.unwrap_or([SECOND_COLUMN_MIN, THIRD_COLUMN_MIN]);
        // The room the grid got on the last pass (its rows' height is all
        // it asks itself): laid out over it, the rows centre in the
        // stretch like the column widths' own late pass.
        let grid_room: f32 = ui
            .ctx()
            .data(|data| data.get_temp(egui::Id::unique(GRID_ROOM_ID)))
            .unwrap_or(0.0);
        // The columns' rects before any draws — General info's takes the
        // room the fixed columns leave (the draft's `1fr`).
        let second_width = saved[0].max(SECOND_COLUMN_MIN);
        let third_width = saved[1].max(THIRD_COLUMN_MIN);
        let height = ui.available_height();
        let first_width = (ui.available_width() - 2.0 * COLUMN_GAP - second_width - third_width)
            .max(first_column_width());
        let mut left = ui.cursor().min.x;
        let top = ui.cursor().min.y;
        let mut next_rect = |width: f32| {
            let rect = egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, height));
            left += width + COLUMN_GAP;
            rect
        };
        let first_rect = next_rect(first_width);
        let second_rect = next_rect(second_width);
        let third_rect = next_rect(third_width);
        // Column 1: General info, Main stats (Captain in its title's
        // row: its click returns the undo merge key and the `edit_team`
        // lands after `panel_with`, since the title row and the fields
        // cannot share `session`), Playable positions.
        let mut first = column(ui, first_rect, |ui| {
            let general = panel(ui, spacing, "General info", None, 0.0, |ui| {
                ui.spacing_mut().item_spacing.y = FIELD_SPACING;
                info_strip::general(ui, session, team_id, slot, player_id, marks, focused);
            });
            let (captained, main) = panel_with(
                ui,
                spacing,
                "Main stats",
                None,
                0.0,
                |ui, title| info_strip::captain_title(ui, title, captain),
                |ui| {
                    ui.spacing_mut().item_spacing.y = FIELD_SPACING;
                    info_strip::main_stats(
                        ui, session, player_id, &stats, &positions, version, marks,
                    );
                },
            );
            if let Some(key) = captained {
                edit_team(session, team_id, key, |team| {
                    team.tactics.set_pieces.captain = slot;
                });
            }
            let positions_panel = panel(
                ui,
                spacing,
                PLAYABLE_POSITIONS_LABEL,
                marks.map(|marks| (marks, live_marks::MarkedField::PlayablePositions)),
                0.0,
                |ui| {
                    playable_positions(
                        ui,
                        session,
                        player_id,
                        &positions,
                        positions_layout,
                        grid_room,
                    );
                },
            );
            vec![general, main, positions_panel]
        });
        // Column 2: Skills (the Cards used badge in its header row), then
        // the ruleset's apart cards in their own panel when it names them,
        // then COM playing styles, their two columns 158 wide.
        let mut second = column(ui, second_rect, |ui| {
            // The cards the ruleset keeps apart.
            let aside = rules
                .as_ref()
                .and_then(|active| aside_cards(active.rules.ruleset()));
            let mut panels = vec![panel_frame(ui, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = spacing;
                    skills_changed |= skills_panel(
                        ui,
                        (
                            rules.as_ref(),
                            aside.as_ref().map(|(_, cards)| cards.as_slice()),
                        ),
                        badge,
                        session,
                        (team_id, slot, marks),
                        &mut skills,
                        refusal,
                    );
                });
            })];
            if let Some((title, aside)) = aside.as_ref() {
                panels.push(panel(
                    ui,
                    spacing,
                    title,
                    marks.map(|marks| (marks, live_marks::MarkedField::AsideCards)),
                    card_box_inset(ui) - CARD_COLS_MARGIN,
                    |ui| {
                        skills_changed |= cards::card_columns(
                            ui,
                            version,
                            &mut skills.skills,
                            &labels::SKILLS,
                            aside,
                            PlayerField::Skill,
                        );
                    },
                ));
            }
            panels.push(panel(
                ui,
                spacing,
                "COM playing styles",
                marks.map(|marks| (marks, live_marks::MarkedField::ComStyles)),
                card_box_inset(ui) - CARD_COLS_MARGIN,
                |ui| {
                    skills_changed |= cards::card_columns(
                        ui,
                        version,
                        &mut skills.com_styles,
                        &labels::COM_STYLES,
                        &all_indices(&labels::COM_STYLES),
                        PlayerField::ComStyle,
                    );
                },
            ));
            panels
        });
        // Column 3: the ability stats, one pair a row.
        let mut third = column(ui, third_rect, |ui| {
            vec![panel(ui, spacing, "Ability stats", None, 0.0, |ui| {
                // The cells' padding after each box and under the last
                // row's boxes (the panel's inset is the one before each
                // label), so the column is the draft's.
                egui::Frame::NONE
                    .inner_margin(egui::Margin {
                        right: CELL_PADDING,
                        bottom: CELL_BOTTOM_PADDING,
                        ..egui::Margin::ZERO
                    })
                    .show(ui, |ui| {
                        ability_stats(ui, session, player_id, &stats, marks);
                    });
            })]
        });
        // The widths the next pass lays the fixed columns out on: this
        // pass's measured ones clamped to the floors, so a sizing pass's
        // narrow measure hands the next pass the floors this pass
        // already allocated.
        let widths = [
            second.rect.width().max(SECOND_COLUMN_MIN),
            third.rect.width().max(THIRD_COLUMN_MIN),
        ];
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::unique(COLUMN_WIDTHS_ID), widths);
        });
        // Discard the pass only when it laid out on other widths than
        // the clamped ones it just measured — the sizing pass's measure
        // clamps to what it allocated, so a new player's first frame
        // stays two passes, not three.
        if [second_width, third_width] != widths {
            ui.ctx()
                .request_discard("the player view's column widths changed");
        }
        // The room the grid's next pass lays out over its rows: the
        // tallest column's bottom less the first column's own — the drawn
        // bottom stands only half the stored room lower (its rows are
        // centred in it), which is all that is taken back; measured with
        // the whole room, the column could never shrink back.
        let own_first_bottom = first.rect.max.y - grid_room / 2.0;
        let row_bottom = own_first_bottom
            .max(second.rect.max.y)
            .max(third.rect.max.y);
        let room = (row_bottom - own_first_bottom).max(0.0);
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::unique(GRID_ROOM_ID), room);
        });
        if room != grid_room {
            ui.ctx()
                .request_discard("the positions grid's room changed");
        }
        // The three columns end on one line: each column's last frame
        // runs down to the tallest's bottom, as the Appearance screen's
        // rows do (`finish_columns`).
        let (second_drawn, third_drawn) = (second.rect.width(), third.rect.width());
        finish_columns(
            ui,
            [
                (&mut first, first_rect.width()),
                (&mut second, second_drawn),
                (&mut third, third_drawn),
            ],
            row_bottom,
        );
        // The row the scroll layout owns: the columns' whole span and the
        // shared bottom, so the next sibling stands under it.
        ui.advance_cursor_after_rect(egui::Rect::from_min_max(
            egui::pos2(first_rect.min.x, top),
            egui::pos2(third_rect.max.x, row_bottom),
        ));
    });
    if skills_changed {
        edit_player(
            session,
            player_id,
            egui::Id::unique(("abilities cards", player_id)).value(),
            move |player| {
                player.skills = skills;
            },
        );
    }
}

/// One of the view's three columns: `add`'s panels' `Prepared`s, each
/// allocated where it was drawn but unpainted until `finish_columns`
/// stretches and paints it, and the column's drawn bounds.
struct Column {
    /// The column's panels, top to bottom.
    panels: Vec<egui::containers::frame::Prepared>,
    /// The column's drawn bounds (its panels' allocated outer rects).
    rect: egui::Rect,
}

/// One of the view's three columns, drawn left to right in `rect` (the
/// rects are known before any draws, so each draws at its own place
/// rather than as the next item in a left-to-right row): its panels
/// `PANEL_GAP` apart and `GROUP_GAP` under the title's line, as the
/// Appearance screen's groups stand.
fn column(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    add: impl FnOnce(&mut egui::Ui) -> Vec<egui::containers::frame::Prepared>,
) -> Column {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing.y = PANEL_GAP;
            ui.add_space(GROUP_GAP);
            let panels = add(ui);
            Column {
                panels,
                rect: ui.min_rect(),
            }
        },
    )
    .inner
}

/// The three columns' frames painted: every panel's stretched to its
/// column's `width` (General info's allocated, the fixed columns' widest
/// panel's, as the draft's `.pcol` children stretch) and each column's
/// last down to `row_bottom`, the tallest column's bottom, so the
/// columns end on one line as the Appearance screen's rows do (the
/// draft's `.pcol > :last-child { flex: 1 }`).
fn finish_columns(ui: &egui::Ui, columns: [(&mut Column, f32); 3], row_bottom: f32) {
    let margin = egui::Frame::group(ui.style()).total_margin().sum();
    for (column, width) in columns {
        let last = column.panels.len() - 1;
        for (at, prepared) in column.panels.iter_mut().enumerate() {
            let mut size = prepared.content_ui.min_rect().size();
            size.x = width - margin.x;
            if at == last {
                // The content's bottom the frame's lower margin above
                // the line (the group's margins are symmetric).
                size.y = row_bottom - margin.y / 2.0 - prepared.content_ui.min_rect().min.y;
            }
            stretch_to(prepared, size);
            prepared.paint(ui);
        }
    }
}

/// One panel of a column, drawn in a `Frame::group` (the Appearance
/// screen's groups' frame) with the `ui.strong` title inside, `TITLE_GAP`
/// above the first thing `add` paints — `first_inset` is how deep into
/// `add`'s first row that thing sits (a card row's painted square is
/// centred in the taller control, `card_box_inset`; a field's frame or a
/// label's text paints from the row's top). The title's row is
/// top-aligned like the Skills header's `horizontal_wrapped`, so every
/// title of the view shares the one line — `ui.horizontal`'s row would
/// centre it in the `interact_size` height and sit it lower. `mark` is
/// Live AATF's held check's with the panel's field: its mark stands
/// after the title. `add` draws through `inset`, `CELL_PADDING` in
/// from the title, `spacing` (the pane's own) restored inside. The
/// returned frame's space is allocated but unpainted: `finish_columns`
/// stretches and paints it.
fn panel(
    ui: &mut egui::Ui,
    spacing: egui::Vec2,
    title: &str,
    mark: Option<(live_marks::Marks<'_>, live_marks::MarkedField)>,
    first_inset: f32,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::containers::frame::Prepared {
    panel_with(ui, spacing, title, mark, first_inset, |_, _| (), add).1
}

/// `panel` with `title_row` drawn on the title's row after the title,
/// given the title's rect to stand on — Main stats' Captain pair. Its
/// result returns with the frame's. What it draws must leave the row a
/// bare title's height (a child `Ui` at a rect, not a widget the row
/// lays out), or every row under moves.
fn panel_with<R>(
    ui: &mut egui::Ui,
    spacing: egui::Vec2,
    title: &str,
    mark: Option<(live_marks::Marks<'_>, live_marks::MarkedField)>,
    first_inset: f32,
    title_row: impl FnOnce(&mut egui::Ui, egui::Rect) -> R,
    add: impl FnOnce(&mut egui::Ui),
) -> (R, egui::containers::frame::Prepared) {
    let mut extra = None;
    let prepared = panel_frame(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = spacing;
            ui.scope_builder(
                egui::UiBuilder::new().layout(egui::Layout::left_to_right(egui::Align::Min)),
                |ui| {
                    let title = ui.strong(title);
                    if let Some((marks, field)) = mark {
                        marks.paint_after(ui, title.rect, field, live_marks::MARK_GAP);
                    }
                    extra = Some(title_row(ui, title.rect));
                },
            );
            ui.add_space(TITLE_GAP - spacing.y - first_inset);
            inset(ui, add);
        });
    });
    (extra.expect("the title row ran"), prepared)
}

/// `add` inside a `Frame::group` begun in `ui`, the frame's space in the
/// column allocated: the returned `Prepared` is unpainted — `paint` it
/// now, or stretch it first when it is its column's last (`finish_columns`).
fn panel_frame(
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::containers::frame::Prepared {
    let mut prepared = egui::Frame::group(ui.style()).begin(ui);
    add(&mut prepared.content_ui);
    prepared.allocate_space(ui);
    prepared
}

/// `add`'s content `CELL_PADDING` in from the title above it, labels and
/// controls alike (the draft's 6-point indent under every title): every
/// panel of the Stats screen and every group of the Appearance screen
/// draws its content through it, so each title reads as the heading of
/// what is under it.
pub(super) fn inset<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: CELL_PADDING,
            ..egui::Margin::ZERO
        })
        .show(ui, add)
        .inner
}

/// How deep into a `cards::card_columns` row its painted square sits
/// (egui centres the `icon_width` box in the control's `interact_size`
/// height): a card panel's title leaves it that much less of `TITLE_GAP`
/// than a first row painted from its top takes; the card columns' own
/// `.skcols` `margin-top` comes off it too (`CARD_COLS_MARGIN`).
fn card_box_inset(ui: &egui::Ui) -> f32 {
    (ui.spacing().interact_size.y - ui.spacing().icon_width) / 2.0
}

/// `end_row` after `index` pairs (1-based) of `per_row` are done, so a row
/// takes as many pairs as `per_row` allows.
pub(crate) fn maybe_end_row(ui: &mut egui::Ui, index: usize, per_row: usize) {
    if index.is_multiple_of(per_row) {
        ui.end_row();
    }
}

/// One small stored field of the Main stats panel or the Appearance
/// screen's Presentation group: the term's name, the drawn label, its
/// schema field, the shown range and the shift between shown and stored
/// (the shifted fields are stored one below the game's numbers, the
/// draft's `+ 1`/`v - 1`).
#[derive(Clone, Copy)]
pub(super) struct SmallStat {
    /// The diff row's name for the field (`terms::row_term`); the control
    /// draws `shown`.
    pub(super) label: &'static str,
    /// The control's drawn label (the weak foot's two share one's:
    /// "Weak Foot: Usage", then "Accuracy").
    pub(super) shown: &'static str,
    /// The schema field gating it.
    pub(super) field: PlayerField,
    /// The shown range.
    pub(super) min: u8,
    /// The shown range's top.
    pub(super) max: u8,
    /// `shown - shift` is the stored value.
    pub(super) shift: u8,
    /// The stored value (`None` on a version that does not store it).
    pub(super) get: fn(&PlayerStats) -> Option<u8>,
    /// The stored write.
    pub(super) set: fn(&mut PlayerStats, u8),
}

/// The Main stats panel's four stored fields, in the draft's order: form
/// 1-8, injury resistance 1-3, weak foot usage and accuracy 1-4 (each
/// stored one lower). Star and playing attitude sit on the Appearance
/// screen, in `appearance::fields::PRESENTATION_STATS`.
pub(super) const SMALL_STATS: [SmallStat; 4] = [
    SmallStat {
        label: "Form",
        shown: "Form",
        field: PlayerField::Form,
        min: 1,
        max: 8,
        shift: 1,
        get: |stats| Some(stats.form),
        set: |stats, value| stats.form = value,
    },
    SmallStat {
        label: "Injury Resistance",
        shown: "Injury Resistance",
        field: PlayerField::InjuryResistance,
        min: 1,
        max: 3,
        shift: 1,
        get: |stats| Some(stats.injury_resistance),
        set: |stats, value| stats.injury_resistance = value,
    },
    SmallStat {
        label: "Weak Foot Usage",
        shown: "Weak Foot: Usage",
        field: PlayerField::WeakFootUsage,
        min: 1,
        max: 4,
        shift: 1,
        get: |stats| Some(stats.weak_foot_usage),
        set: |stats, value| stats.weak_foot_usage = value,
    },
    SmallStat {
        label: "Weak Foot Accuracy",
        shown: "Accuracy",
        field: PlayerField::WeakFootAccuracy,
        min: 1,
        max: 4,
        shift: 1,
        get: |stats| Some(stats.weak_foot_accuracy),
        set: |stats, value| stats.weak_foot_accuracy = value,
    },
];

/// One `SmallStat`'s label/control pair: `number` for a stored field, the
/// `-` stand-in for a gated one (the label stays too: the gated control
/// keeps its cell). The Appearance screen's Presentation group uses it;
/// the Main stats panel's `small` pairs draw the halves apart.
pub(super) fn small_stat(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    stats: &PlayerStats,
    small: &SmallStat,
) {
    match (small.get)(stats) {
        Some(stored) => {
            number(
                ui,
                session,
                player_id,
                small.shown,
                stored + small.shift,
                small.min..=small.max,
                move |player, shown| {
                    (small.set)(&mut player.stats, shown - small.shift);
                },
            );
        }
        None => {
            gated_label(ui, small.shown, gating_tooltip(small.field));
            gated_number(ui, gating_tooltip(small.field));
        }
    }
}

/// A positions cell's width (the draft's `.poscell` 62px).
const POS_CELL_WIDTH: f32 = 62.0;
/// A rating square's side (the draft's `.rate .sel` 17px).
const RATING_SIDE: f32 = 17.0;
/// The gap between a cell's rating squares (the draft's `.rate` 2px).
const RATING_GAP: f32 = 2.0;
/// A cell's three rating squares' width.
const RATINGS_WIDTH: f32 = 3.0 * RATING_SIDE + 2.0 * RATING_GAP;
/// The gap between a row's cells (the draft's `#pos-grid .posrow` 4px).
const POS_CELL_GAP: f32 = 4.0;
/// The gap between a cell's label and its rating squares (the draft's
/// `.plabel` 2px margin under it).
const POS_LABEL_GAP: f32 = 2.0;
/// The gap between the grid's rows: the draft's `.poscell` padding, 2
/// above and 3 below, and the `#pos-grid` 4px gap between them.
const POS_ROW_GAP: f32 = 9.0;
/// The `#pos-layout`'s own `margin-top` over a first field row's (the
/// draft's 4.5 against ~3): the positions' selector stands this much
/// lower under the panel's title than a field's row.
const POS_LAYOUT_MARGIN: f32 = 1.5;
/// The gap between the layout toggle and the grid under it: the draft's
/// `#pos-layout` bottom to its `#pos-grid` top (7), plus the half point
/// by which the build's toggle stands higher than the draft's.
pub(super) const TOGGLE_GRID_GAP: f32 = 7.5;
/// The gap between the List layout's rows: a cell's padding, 1 above and
/// 1 below, and the `#pos-grid.list` 1px gap between them.
const LIST_ROW_GAP: f32 = 3.0;
/// The List layout's order (the draft's `POS_LIST`): the forwards, the
/// wide and attacking midfielders, the central ones, the full backs
/// before the centre back, the keeper.
const POSITION_LIST: [&str; 13] = [
    "CF", "SS", "LWF", "RWF", "AMF", "LMF", "RMF", "CMF", "DMF", "LB", "RB", "CB", "GK",
];
/// A List row's label cell's width (the draft's `.plabel` 34px).
const LIST_LABEL_WIDTH: f32 = 34.0;
/// A List row's cell padding on each side (the draft's `.poscell` 1px
/// 3px; its 1 above and below only widens the registered outline).
const LIST_CELL_PAD: f32 = 3.0;
/// The gap between a List row's label cell and its ratings (the draft's
/// `.plabel` 6px margin right).
const LIST_LABEL_GAP: f32 = 6.0;
/// A List row's whole cell width: the padding, the label cell, the gap
/// and the three rating squares.
const LIST_CELL_WIDTH: f32 =
    2.0 * LIST_CELL_PAD + LIST_LABEL_WIDTH + LIST_LABEL_GAP + RATINGS_WIDTH;

/// The chosen rating's fill and text colours (the draft's `.sel.on`
/// variants): A a deep green under a white letter, B the selection colour
/// with its foreground, C a grey square with the normal text — named
/// outright, since a selected button's own text colour is the
/// selection's foreground.
fn rating_colors(ui: &egui::Ui, rating: u8) -> (egui::Color32, egui::Color32) {
    let palette = studio_core::theme::palette(ui.visuals());
    match rating {
        2 => (palette.rating_a_fill, palette.rating_a_text),
        1 => (
            ui.visuals().selection.bg_fill,
            ui.visuals().selection.stroke.color,
        ),
        _ => (palette.rating_c_fill, ui.visuals().text_color()),
    }
}

/// `name`'s position byte (its `POSITIONS` index, as `registered` stores
/// it) and its `playable` index — the label's index is not the array's.
fn playable_index(name: &str) -> (usize, usize) {
    let index = POSITIONS
        .iter()
        .position(|position| *position == name)
        .expect("every grid position is a POSITIONS name");
    let at = PLAYABLE_POSITIONS
        .iter()
        .position(|&position| position == index)
        .expect("PLAYABLE_POSITIONS is a permutation of 0..13");
    (index, at)
}

/// A `POSITION_ROWS` row's drawn width: its cells and the gaps between.
fn row_width(row: &[&str]) -> f32 {
    row.len() as f32 * POS_CELL_WIDTH + (row.len() - 1) as f32 * POS_CELL_GAP
}

/// The `Pitch` `Left` `List` toggle under the panel's title (the draft's
/// `#pos-layout`), left-aligned, each segment its word padded 5 a side
/// (`.rate .sel`): the held one filled in the quiet grey of the Stronger
/// foot toggle's, not the selection's blue — the choice is set once and
/// forgotten, and the accent would pull the eye to it. `layout` is the
/// `positions_layout` setting's view field: a press on an unheld segment
/// holds it, on the held one it writes nothing.
fn layout_toggle(ui: &mut egui::Ui, layout: &mut PositionsLayout) {
    // A Button's text is the Body style's here (the widget's own text
    // style), the size the segments' galleys are measured at.
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut height = 0.0f32;
    let segments = PositionsLayout::CHOICES.map(|choice| {
        let galley = ui.painter().layout_no_wrap(
            choice.label().to_owned(),
            font.clone(),
            egui::Color32::PLACEHOLDER,
        );
        height = height.max(galley.size().y);
        // The draft's `.rate .sel` 5-point padding either side of the word.
        (choice, choice.label(), galley.size().x + 10.0)
    });
    // The row a line of text tall (the draft gives its segments no
    // vertical padding): `segment_row` sizes its buttons to
    // `interact_size.y`, set to their words' height for the row and
    // restored for the grid's rows.
    let interact = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = height;
    if let Some((picked, _)) = info_strip::segment_row(ui, &segments, *layout, true) {
        *layout = picked;
    }
    ui.spacing_mut().interact_size.y = interact;
}

/// The 13 positions in the selected `layout` (the draft's `POS_LAYOUT`),
/// behind `layout_toggle`: `Pitch` and `Left` draw `POSITION_ROWS` (the
/// rows the Tactics pitch's formation name reads too), each cell the
/// position's label above the three `C` `B` `A` ratings (stored 0, 1, 2;
/// a stored 3 selects none), the cell's contents centred so the label
/// sits over the B — each Pitch row centred in the panel, the Left
/// rows starting at the block's left edge, the block as wide as the
/// widest row itself centred (`#pos-grid.left`); `List` draws
/// `POSITION_LIST` one position a row, the label in a fixed-width cell
/// left of its ratings, the column centred (`#pos-grid.list`). The
/// registered position's cell is outlined in the selection colour (the
/// draft's `.poscell.reg`) in every layout. `playable[i]` is
/// `POSITIONS[PLAYABLE_POSITIONS[i]]`'s rating. The grid takes the
/// layout's own rows' height plus `room`, the stretch the tallest
/// column gave it on the last pass (`GRID_ROOM_ID`): Pitch and Left
/// leave the panel shorter than List does, and the rows stand centred
/// in the whole room under the toggle (the draft's `#pos-grid`
/// `justify-content: center`), not at its top.
fn playable_positions(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    positions: &PlayerPositions,
    layout: &mut PositionsLayout,
    room: f32,
) {
    // `item_spacing.y` is consumed by the advance after each drawn row:
    // the gap under the toggle stands before `layout_toggle`, the rows'
    // own pitch before theirs.
    ui.spacing_mut().item_spacing.y = TOGGLE_GRID_GAP;
    ui.add_space(POS_LAYOUT_MARGIN);
    layout_toggle(ui, layout);
    let mut playable = positions.playable;
    // The rows centre in the panel's content span. `ui` is the panel's
    // inset content, so the frame's left stands `CELL_PADDING` left of
    // it and a row's lead space is that much shorter.
    let padding = f32::from(CELL_PADDING);
    let column_width = ui.available_width() + padding;
    let lead_x = |width: f32| ((column_width - width) / 2.0 - padding).max(0.0);
    let row_gap = match *layout {
        PositionsLayout::Pitch | PositionsLayout::Left => POS_ROW_GAP,
        PositionsLayout::List => LIST_ROW_GAP,
    };
    let rows = match *layout {
        PositionsLayout::Pitch | PositionsLayout::Left => {
            // A cell's drawn height: the label galley's (or the row's
            // `interact_size` where it is taller), the label-to-ratings
            // gap and the squares.
            let font = egui::TextStyle::Body.resolve(ui.style());
            let label_h = ui
                .painter()
                .layout_no_wrap("GK".to_owned(), font, egui::Color32::PLACEHOLDER)
                .size()
                .y
                .max(ui.text_style_height(&egui::TextStyle::Body));
            let cell = label_h + POS_LABEL_GAP + RATING_SIDE;
            POSITION_ROWS.len() as f32 * cell + (POSITION_ROWS.len() - 1) as f32 * POS_ROW_GAP
        }
        PositionsLayout::List => {
            POSITION_LIST.len() as f32 * RATING_SIDE
                + (POSITION_LIST.len() - 1) as f32 * LIST_ROW_GAP
        }
    };
    // The rows stand centred in the grid's room (the draft's
    // `justify-content: center`): `add_space` advances the cursor
    // outright, no item spacing.
    let lead = room / 2.0;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), rows + room),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.add_space(lead);
            ui.spacing_mut().item_spacing.y = row_gap;
            match *layout {
                PositionsLayout::Pitch | PositionsLayout::Left => {
                    // Left starts every row at the block's left edge; Pitch
                    // centres each row's own width.
                    let widest = POSITION_ROWS
                        .iter()
                        .map(|row| row_width(row))
                        .fold(0.0_f32, f32::max);
                    for row in POSITION_ROWS {
                        let width = if *layout == PositionsLayout::Left {
                            widest
                        } else {
                            row_width(row)
                        };
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = POS_CELL_GAP;
                            ui.add_space(lead_x(width));
                            for name in row {
                                grid_cell(ui, session, player_id, positions, &mut playable, name);
                            }
                        });
                    }
                }
                PositionsLayout::List => {
                    // The row a rating square tall, as the draft's list cells
                    // are; the column of them centred in the panel.
                    ui.spacing_mut().interact_size.y = RATING_SIDE;
                    let pad = lead_x(LIST_CELL_WIDTH);
                    for name in POSITION_LIST {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.add_space(pad);
                            list_row(ui, session, player_id, positions, &mut playable, name);
                        });
                    }
                }
            }
        },
    );
}

/// One `POSITION_ROWS` cell (the draft's `.poscell`): the position's
/// label above its three ratings, both centred in the 62-wide cell, the
/// registered position's outlined around it.
fn grid_cell(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    positions: &PlayerPositions,
    playable: &mut [u8; 13],
    name: &'static str,
) {
    let (index, at) = playable_index(name);
    let cell = ui.vertical(|ui| {
        // The draft's `.poscell` 62px and `.rate`'s squares 2 apart:
        // `Button::selectable` is what `selectable_value` draws.
        ui.set_width(POS_CELL_WIDTH);
        // The label's row one line of text high and the squares
        // `RATING_SIDE` high, as the draft's: egui would make either row
        // at least a control's height.
        ui.spacing_mut().interact_size.y = ui.text_style_height(&egui::TextStyle::Body);
        ui.spacing_mut().item_spacing.y = POS_LABEL_GAP;
        // The cell's contents centred (the draft's `align-items:
        // center`): the label padded to the cell's centre, so over the
        // B; the squares 3.5 in.
        let galley = ui.painter().layout_no_wrap(
            name.to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            egui::Color32::PLACEHOLDER,
        );
        ui.horizontal(|ui| {
            ui.add_space(((POS_CELL_WIDTH - galley.size().x) / 2.0).max(0.0));
            let label = ui
                .add(
                    egui::Label::new(name)
                        .sense(egui::Sense::click())
                        .selectable(false),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(format!("Make {name} the registered position"));
            register_on_click(&label, session, player_id, index, positions);
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = RATING_GAP;
            ui.spacing_mut().button_padding = egui::vec2(0.0, 0.0);
            ui.add_space((POS_CELL_WIDTH - RATINGS_WIDTH) / 2.0);
            rating_squares(ui, session, player_id, playable, at);
        });
    });
    if index == usize::from(positions.registered) {
        registered_outline(ui, cell.response.rect.expand(2.0));
    }
}

/// One `POSITION_LIST` row (the draft's `#pos-grid.list .poscell`): the
/// position's label at the left of a fixed-width cell, its `C` `B` `A`
/// at the right — the whole label cell the click target, as the draft's
/// `.plabel` is — the registered position's outlined around the cell
/// and its padding.
fn list_row(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    positions: &PlayerPositions,
    playable: &mut [u8; 13],
    name: &'static str,
) {
    let (index, at) = playable_index(name);
    let cell_min = ui.cursor().min;
    ui.add_space(LIST_CELL_PAD);
    // The label cell's click target is its whole box, so it is a hand-
    // drawn galley over an interact rather than a Label widget (whose
    // click region would be the text alone).
    let (label_rect, cell) = ui.allocate_exact_size(
        egui::vec2(LIST_LABEL_WIDTH, ui.spacing().interact_size.y),
        egui::Sense::click(),
    );
    let cell = cell
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!("Make {name} the registered position"));
    cell.widget_info(|| {
        egui::WidgetInfo::labeled(egui::accesskit::Role::Label, true, name.to_owned())
    });
    register_on_click(&cell, session, player_id, index, positions);
    let galley = ui.painter().layout_no_wrap(
        name.to_owned(),
        egui::TextStyle::Body.resolve(ui.style()),
        egui::Color32::PLACEHOLDER,
    );
    ui.painter().galley(
        egui::pos2(
            label_rect.min.x,
            label_rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        ui.style().interact(&cell).text_color(),
    );
    ui.add_space(LIST_LABEL_GAP);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = RATING_GAP;
        ui.spacing_mut().button_padding = egui::vec2(0.0, 0.0);
        rating_squares(ui, session, player_id, playable, at);
    });
    if index == usize::from(positions.registered) {
        // Around the whole cell, padding included (the draft's outline
        // sits on the `.poscell` box): the cell's 1 point above and
        // below the squares' row.
        let cell_rect = egui::Rect::from_min_size(
            cell_min - egui::vec2(0.0, 1.0),
            egui::vec2(LIST_CELL_WIDTH, RATING_SIDE + 2.0),
        );
        registered_outline(ui, cell_rect);
    }
}

/// `response`'s click makes `index` the registered position (the draft's
/// `.plabel`, the same undoable edit as General info's Registered
/// position combo); on the registered one's own label it writes nothing,
/// so no empty undo entry.
fn register_on_click(
    response: &egui::Response,
    session: &mut Session,
    player_id: u32,
    index: usize,
    positions: &PlayerPositions,
) {
    if response.clicked() && index != usize::from(positions.registered) {
        edit_player(session, player_id, response.id.value(), |player| {
            player.positions.registered = u8::try_from(index).expect("a POSITIONS index is a byte");
        });
    }
}

/// The position `at`'s `C` `B` `A` squares (the draft's `.rate`): the
/// chosen one filled (`rating_colors`), a click on another writing
/// `playable` back as one undoable edit. The letters not chosen draw
/// the normal text colour, darker than a resting button's own grey, so
/// the chosen one stands out (the draft's `.rate:not(.off)
/// .sel:not(.on)`): the row's resting colour is set to it inside, where
/// only these squares draw, and a hover still takes the hovered state's
/// brighter grey.
fn rating_squares(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    playable: &mut [u8; 13],
    at: usize,
) {
    ui.visuals_mut().widgets.inactive.fg_stroke.color = ui.visuals().text_color();
    let unheld = studio_core::theme::unheld_edge(ui.visuals());
    for (label, rating) in RATING_LABELS {
        let on = playable[at] == rating;
        // The chosen square's edge is its fill's (`edge_color`), an
        // unchosen one's the panel's (`unheld_edge`).
        let (response, edge) = if on {
            // The chosen rating filled (the draft's `.sel.on.*`).
            let (fill, text) = rating_colors(ui, rating);
            (
                ui.add(
                    egui::Button::selectable(true, egui::RichText::new(label).color(text))
                        .min_size(egui::vec2(RATING_SIDE, RATING_SIDE))
                        .fill(fill),
                ),
                studio_core::theme::edge_color(ui.visuals(), fill),
            )
        } else {
            (
                ui.add(
                    egui::Button::selectable(false, label)
                        .min_size(egui::vec2(RATING_SIDE, RATING_SIDE)),
                ),
                unheld,
            )
        };
        resting_edge(ui, &response, edge);
        if response.clicked() && playable[at] != rating {
            playable[at] = rating;
            let written = *playable;
            edit_player(session, player_id, response.id.value(), |player| {
                player.positions.playable = written;
            });
        }
    }
}

/// The registered position's cell outline (the draft's `.poscell.reg`):
/// the selection colour's stroke outside `rect`, rounded as the widgets'
/// frames are.
fn registered_outline(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_stroke(
        rect,
        ui.visuals().widgets.inactive.corner_radius,
        egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
        egui::StrokeKind::Outside,
    );
}

/// The 25 stats of `ABILITY_STATS` in one column in the game's order (the
/// view's third column), each label beside its `DragValue` 40-99 (the
/// draft's `td.ilabel`): the labels start at one x, the grid's first
/// column, and the numbers stand in its second, so a short label stands
/// far from its number; a stat the version lacks shows `-`, disabled,
/// its name dimmed with it as a version-gated card's is.
/// The grid sizes the label column to the widest label itself: a new
/// grid's first pass is an invisible sizing pass, so the numbers stand at
/// their final x from the first frame drawn. `marks` is Live AATF's held
/// check's: a stat a violation is about carries its mark before its
/// label's left edge, over the cell padding before it; every label starts
/// at the column's left edge, so the stats' marks share one x whatever
/// the labels' lengths.
fn ability_stats(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    stats: &PlayerStats,
    marks: Option<live_marks::Marks<'_>>,
) {
    let abilities = stats.abilities();
    let version = session.version();
    egui::Grid::new("abilities stats")
        .num_columns(2)
        .spacing(egui::vec2(info_strip::LABEL_GAP, FIELD_SPACING))
        .show(ui, |ui| {
            for (index, &label) in ABILITY_STATS.iter().enumerate() {
                let name = stat_name(version, label);
                // A stat the version lacks reads dimmed with its `-`, as a
                // version-gated card's name is, under the same tooltip.
                let label_cell = if abilities[index].is_some() {
                    ui.label(name).rect
                } else {
                    gated_label(ui, &name, gating_tooltip(ABILITY_FIELDS[index])).rect
                };
                match abilities[index] {
                    Some(value) => {
                        number_field(
                            ui,
                            session,
                            player_id,
                            value,
                            40..=99,
                            move |player, shown| player.stats.set_ability(index, shown),
                        );
                        if let Some(marks) = marks {
                            let stat = aatf::Stat::try_from(label.to_owned())
                                .expect("ABILITY_STATS are stat names");
                            marks.paint(ui, label_cell, live_marks::MarkedField::Stat(stat));
                        }
                    }
                    // The gated control keeps its cell too.
                    None => {
                        gated_number(ui, gating_tooltip(ABILITY_FIELDS[index]));
                    }
                }
                ui.end_row();
            }
        });
}

/// The Skills panel's content: the header row's Cards used badge — the
/// player's own tier's, the tier `Rules::player_tier` reads him as, then
/// that tier's `card_limits`; no tier matching draws the stand-in, and a
/// check that fails on him draws the red click line that opens the
/// error in `refusal`, then the checkboxes in their two columns, the
/// skills but the `aside` indices (the aside panel's own).
/// Returns whether a card was ticked this frame. The badge runs on a
/// worker thread under `cards`'s `BadgeKey`
/// (`worker::WorkerResult`'s cycle) — a `Session::revision` bump, a
/// selection change or another ruleset re-asks it — and until the answer
/// lands the kept one is drawn: the same player's (only the revision
/// moved), nothing for another player or ruleset. `marks` is Live AATF's
/// held check's: the header's mark, the skills' and the card budget's in
/// one, stands after the header row's last item.
fn skills_panel(
    ui: &mut egui::Ui,
    (rules, aside): (Option<&ActiveRules>, Option<&[u8]>),
    badge: &mut Cards,
    session: &mut Session,
    (team_id, slot, marks): (u32, u8, Option<live_marks::Marks<'_>>),
    skills: &mut PlayerSkills,
    refusal: &mut Option<Refusal>,
) -> bool {
    let held_badge = rules.and_then(|active| {
        let key = BadgeKey {
            revision: session.revision(),
            team: team_id,
            slot,
            ruleset: active.key.clone(),
        };
        // One run in flight: a key that moves while a run is up waits for
        // its reply (`settled` stays off, so `update` keeps polling
        // without spawning), and the frame the reply lands on starts the
        // queued run. A spawn waits for no settled edit: a wheel notch is
        // a whole edit.
        let can_start = badge.pending.is_none();
        let waiting = badge.update(&key, true, can_start, |key| {
            spawn_badge(ui.ctx(), session.file(), &active.rules, key.clone())
        });
        if waiting && badge.pending.is_none() {
            // A run is due but none could start on this frame (the
            // in-flight run's reply landed inside `update`): the next
            // frame starts it, so ask for the frame rather than wait for
            // input.
            ui.ctx().request_repaint();
        }
        badge
            .result
            .as_ref()
            .filter(|(held, _)| {
                held.team == key.team && held.slot == key.slot && held.ruleset == key.ruleset
            })
            .map(|(_, own)| own)
    });
    // A tier check that fails on him draws `Cards used` then a red click
    // line in the badge's place, opening its error in the refusal dialog
    // (the AATF check never runs the two functions the badge asks, so a
    // member's edit that breaks only them would otherwise show nowhere).
    let tier_error = if let Some(OwnBadge::Failed(error)) = held_badge {
        Some(error.clone())
    } else {
        None
    };
    // The header's row one line of text high, as the draft's: a wrapping
    // row is otherwise a control's height, which stood the badge's text and
    // every card under the title lower than the draft's.
    let control_height = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = ui.text_style_height(&egui::TextStyle::Body);
    let mut title_rect = egui::Rect::NOTHING;
    let header = ui.horizontal_wrapped(|ui| {
        title_rect = ui.strong("Skills").rect;
        if let Some(error) = &tier_error {
            ui.label("Cards used:");
            let response = ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(TIER_CHECK_FAILED_LINE)
                            .color(ui.visuals().error_fg_color),
                    )
                    .sense(egui::Sense::click()),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                *refusal = Some(Refusal {
                    title: "The tier check failed",
                    message: error.clone(),
                });
            }
        } else {
            match held_badge {
                Some(OwnBadge::Tier { label, limits }) => cards::cards_used(
                    ui,
                    &cards::CardsBadge::Tier(cards::TierCards { label, limits }),
                ),
                Some(OwnBadge::NoTier) => cards::cards_used(ui, &cards::CardsBadge::NoTier),
                // A failing held badge is the error line above; another
                // player's or ruleset's draws nothing until its own lands.
                Some(OwnBadge::Failed(_)) | None => {}
            }
        }
    });
    // The header's mark stands after the row's last item — the badge,
    // its stand-in or the title — the row's item spacing off it, as the
    // next item would stand: after the title it would overlap the badge,
    // before the badge it would shift the badge whenever one showed.
    if let Some(marks) = marks {
        marks.paint_after(
            ui,
            title_rect.with_max_x(header.response.rect.max.x),
            live_marks::MarkedField::Skills,
            ui.spacing().item_spacing.x,
        );
    }
    ui.spacing_mut().interact_size.y = control_height;
    // The title stands `TITLE_GAP` above the first card row's painted
    // square, as `panel`'s do, the card columns' own margin with it.
    ui.add_space(TITLE_GAP - ui.spacing().item_spacing.y - card_box_inset(ui) + CARD_COLS_MARGIN);
    let shown: Vec<u8> = all_indices(&labels::SKILLS)
        .into_iter()
        .filter(|index| !aside.is_some_and(|aside| aside.contains(index)))
        .collect();
    inset(ui, |ui| {
        cards::card_columns(
            ui,
            session.file().version(),
            &mut skills.skills,
            &labels::SKILLS,
            &shown,
            PlayerField::Skill,
        )
    })
}

/// `names`' whole index range, a `card_columns` `shown` when no panel
/// splits the kind.
fn all_indices(names: &[&'static str]) -> Vec<u8> {
    (0..u8::try_from(names.len()).expect("a label table is byte-indexed")).collect()
}

/// The title and the skill indices the aside panel lists, in `SKILLS`
/// order with none twice: the ruleset's `tricks` card group ("Tricks"),
/// else every skill one of its `free_cards` effects names — those free
/// only for a tier, a position or the captain included ("Free cards"),
/// since a card moving between panels as a player's tier changes would
/// be harder to find. `None` when the ruleset names no such card: every
/// skill then stays under Skills.
fn aside_cards(ruleset: &aatf::Ruleset) -> Option<(&'static str, Vec<u8>)> {
    let mut indices: Vec<u8> = ruleset
        .card_groups
        .get("tricks")
        .into_iter()
        .flatten()
        .filter_map(skill_index)
        .collect();
    if !indices.is_empty() {
        indices.sort_unstable();
        indices.dedup();
        return Some(("Tricks", indices));
    }
    indices.extend(
        ruleset
            .specials
            .iter()
            .flat_map(|special| std::iter::once(&special.then).chain(special.then.choice.iter()))
            .flat_map(|effects| effects.free_cards.iter())
            .filter_map(skill_index),
    );
    indices.sort_unstable();
    indices.dedup();
    (!indices.is_empty()).then_some(("Free cards", indices))
}

/// `card`'s index when it is a skill; a COM style is no aside card (it
/// stays under COM playing styles).
fn skill_index(card: &aatf::Card) -> Option<u8> {
    match card {
        aatf::Card::Skill(index) => Some(*index),
        aatf::Card::ComStyle(_) => None,
    }
}

/// The Skills header's Cards used badge (`own_badge`) for a `BadgeKey`:
/// the last answer with its key and the run in flight
/// (`worker::WorkerResult`'s poll-and-spawn cycle).
pub(super) type Cards = WorkerResult<BadgeKey, OwnBadge>;

/// One badge run's worker: `key`'s team viewed on the caller's thread
/// (the `EditFile` lives there), then `player_tier` and `card_limits` on
/// the shared ruleset off it, the key riding the reply and a repaint
/// asked for with it.
fn spawn_badge(
    ctx: &egui::Context,
    file: &EditFile,
    rules: &Arc<aatf::Rules>,
    key: BadgeKey,
) -> Pending<BadgeKey, OwnBadge> {
    let team = TeamView::new(file, key.team).map_err(|error| error.to_string());
    let (sender, replies) = crossbeam_channel::unbounded();
    let rules = Arc::clone(rules);
    let ctx = ctx.clone();
    let reply_key = key.clone();
    let slot = key.slot;
    std::thread::spawn(move || {
        let badge = match team {
            Ok(team) => own_badge(&rules, &team, slot),
            Err(error) => OwnBadge::Failed(error),
        };
        // A newer run can replace the receiver mid-flight; that send's
        // failure is the superseded thread ending, not an error.
        if sender.send((reply_key, badge)).is_err() {
            log::debug!("the badge run's caller dropped a superseded run");
        }
        ctx.request_repaint();
    });
    Pending { key, replies }
}

/// What `rules` read `team`'s roster `slot` as: his own tier with its
/// card economy, no tier, or a check that failed with its error (an
/// empty slot fails too). The two script calls run on the badge run's
/// worker thread: each reads the whole team under every candidate
/// height system.
fn own_badge(rules: &aatf::Rules, team: &TeamView, slot: u8) -> OwnBadge {
    let Some(player) = team.player(slot) else {
        // The slot as the player list numbers it, from 01.
        return OwnBadge::Failed(format!(
            "slot {:02} of the roster holds no player",
            u32::from(slot) + 1
        ));
    };
    match rules.player_tier(team, player) {
        Ok(Some(tier)) => match rules.card_limits(team, player, &tier.id) {
            Ok(limits) => OwnBadge::Tier {
                label: tier.label.clone(),
                limits,
            },
            Err(error) => OwnBadge::Failed(error.to_string()),
        },
        Ok(None) => OwnBadge::NoTier,
        Err(error) => OwnBadge::Failed(error.to_string()),
    }
}

/// A `label`/`DragValue` pair over `shown` in `range`; on a change `store`
/// maps the shown value onto the player's stored one.
/// `clamp_existing_to_range` stays off: an egui number control clamps the
/// value it draws unless told not to, and a stored value past the range
/// would come back changed on the frame that drew it (drawing a player
/// never edits him).
pub(crate) fn number<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    label: impl Into<egui::WidgetText>,
    shown: T,
    range: RangeInclusive<T>,
    store: impl FnOnce(&mut PlayerEntry, T),
) -> egui::Response {
    ui.label(label);
    number_field(ui, session, player_id, shown, range, store)
}

/// `number`'s `DragValue` alone, for a caller drawing the label itself
/// (`ability_stats` wraps the field cell in the stat's `scoped` scope, so
/// the label keeps its own cell while the field keeps a stable id across
/// column counts).
fn number_field<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    shown: T,
    range: RangeInclusive<T>,
    store: impl FnOnce(&mut PlayerEntry, T),
) -> egui::Response {
    let mut shown = shown;
    let mut response = number_arrows(ui, &mut shown, range.clone(), |ui, value| {
        ui.add(number_box(
            egui::DragValue::new(value)
                .range(range.clone())
                .clamp_existing_to_range(false),
        ))
    });
    if step_number(&mut shown, number_wheel_steps(ui, &response), range) {
        response.mark_changed();
    }
    if response.changed() {
        edit_player(session, player_id, response.id.value(), |player| {
            store(player, shown);
        });
    }
    response
}

/// The disabled stand-in for a number the save's version does not store:
/// `-` inside the control's usual bounds (a Button at the DragValue's
/// minimum size, so an enabled version's control takes the same rect,
/// SE-GUI-06) drawn as a disabled number box, under `tooltip` (the `PES
/// NN+` tooltip, as `gated_label`'s).
pub(super) fn gated_number(ui: &mut egui::Ui, tooltip: String) {
    number_box_look(ui, |ui| {
        add_enabled(
            ui,
            false,
            egui::Button::new("-").min_size(ui.spacing().interact_size),
        )
    })
    .on_disabled_hover_text(tooltip);
}

/// The label of a field the save's version does not store: dimmed as its
/// disabled control is, under the control's `PES NN+` tooltip, so the row
/// reads as a field this game does not have rather than one left to
/// fill. `Sense::CLICK` sends the label's text through the widget colours
/// `add_enabled` dims (a hover-sensed label paints the plain text colour);
/// `CLICK` alone, not `Sense::click()`, whose focus bit would offer a
/// focus the disabled label can never take.
pub(crate) fn gated_label(ui: &mut egui::Ui, text: &str, tooltip: String) -> egui::Response {
    add_enabled(ui, false, egui::Label::new(text).sense(egui::Sense::CLICK))
        .on_disabled_hover_text(tooltip)
}

/// The tooltip naming the first version that stores `field` (`PES NN+`).
pub(crate) fn gating_tooltip(field: PlayerField) -> String {
    format!(
        "PES {}+",
        first_version_with(field)
            .expect("a gated field is stored by some version")
            .number()
    )
}

/// The registered position's full names in `POSITIONS` order (the draft's
/// `POS_FULL`, one spelling throughout — not the Team TOML/TSV's own
/// table, whose mixed "Center Back" and "Centre Forward" the parity
/// output keeps).
pub(super) const POSITIONS_FULL: [&str; 13] = [
    "Goalkeeper",
    "Centre Back",
    "Left Back",
    "Right Back",
    "Defensive Midfielder",
    "Centre Midfielder",
    "Left Midfielder",
    "Right Midfielder",
    "Attacking Midfielder",
    "Left Wing Forward",
    "Right Wing Forward",
    "Second Striker",
    "Centre Forward",
];

/// `stored`'s position as the registered-position combo shows it,
/// `<abbr> - <full name>` (`CB - Centre Back`, British spelling), `?`
/// past the table.
pub(super) fn position_label(stored: u8) -> String {
    match POSITIONS.get(usize::from(stored)) {
        Some(abbr) => format!("{abbr} - {}", POSITIONS_FULL[usize::from(stored)]),
        None => "?".to_owned(),
    }
}

/// `stored`'s foot/hand label (`FOOT` title-cased), its number when past
/// the table.
pub(super) fn foot_label(stored: u8) -> String {
    FOOT.get(usize::from(stored))
        .map(|name| title_case(name))
        .unwrap_or_else(|| stored.to_string())
}

#[cfg(test)]
mod tests;
