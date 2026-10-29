//! The player view's three columns (the draft's `playerView`/`.pcols`): General
//! info (`info_strip::general`), Other stats and Playable positions in
//! the first, 350 wide; Skills with the Cards left badge in its header
//! row, the ruleset's apart cards (its `tricks` group, else the skills
//! its `free_cards` effects name) and COM playing styles in the second;
//! the 25 ability stats in the
//! third — one gap apart (`view::show` splits the room the columns leave
//! into four equal gaps), tops aligned, wrapping to the next line where
//! the pane cannot hold the next column. Star and playing attitude, which
//! change only cutscenes, sit in the Appearance section's Cutscenes group
//! (`appearance::CUTSCENE_STATS`); the quick actions sit under the player
//! list (`quick_actions`).
//! Every control edits a copy of the player's entry and, when it changes,
//! pushes it with `Session::edit_player` under the control's `egui::Id`
//! value, as `info_strip.rs` does; edit flags are not touched. A field
//! the save's version does not store is drawn in its place disabled with
//! a `PES NN+` tooltip naming the first version that has it.

use std::ops::RangeInclusive;

use aatf::TeamView;
use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::{
    self, ABILITY_STATS, FOOT, PLAYABLE_POSITIONS, POSITIONS, title_case,
};
use pes_savefile::model::{PlayerEntry, PlayerPositions, PlayerSkills, PlayerStats};
use pes_savefile::schema::fields::{ABILITY_FIELDS, PlayerField};
use pes_savefile::schema::first_version_with;
use studio_core::widgets::{step_number, wheel_steps};
use team_widgets::cards;
use team_widgets::pitch::POSITION_ROWS;

use crate::session::Session;
use crate::view::{
    ActiveRules, BadgeKey, OwnBadge, Refusal, SaveView, edit_player, info_strip, live_marks,
    selected_player,
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

/// The line a tier check's failure draws in the badge's place: it opens
/// the error in the refusal dialog (the quick actions' line's shape).
const TIER_CHECK_FAILED_LINE: &str = "Tier check failed, click for more information.";

/// The gap between the panels of one column (the draft's `.pcol` `gap`),
/// and between Skills' two card columns (`cards` keeps the same 14).
const PANEL_GAP: f32 = 14.0;
/// The gap a panel's title stands above the first thing drawn under it —
/// a field's frame, a checkbox's painted square or a label row's text:
/// the team view's titles' spacing, so the two views read alike.
const TITLE_GAP: f32 = 6.0;
/// The padding of the ability stats' cells, either side of a label and
/// of a box (the draft's `stats-grid` `td`), and how far General info's
/// and Other stats' labels and fields stand in from their titles (the
/// draft's `.pcols .gen` `padding-left`), so a panel's title heads its
/// body.
const CELL_PADDING: i8 = 6;

/// The first column's width (the draft's `.pcol.first` 350px): General
/// info's fields and their inset.
fn first_column_width() -> f32 {
    info_strip::GENERAL_WIDTH + f32::from(CELL_PADDING)
}

/// The egui memory id `show` writes each column's measured width to each
/// pass (the view's gaps and the columns' own widths use the previous
/// pass's; a pass that measures different ones is discarded).
pub(super) const COLUMN_WIDTHS_ID: &str = "save_editor player column widths";

/// The three columns' measured widths, summed, without the gaps between
/// them: `view::show`'s gaps read the previous pass's (with none, the
/// pane's: that pass draws left-aligned and is discarded, so the room the
/// fixed columns leave splits evenly on the screen).
pub(crate) fn columns_width(ui: &egui::Ui) -> f32 {
    ui.ctx()
        .data(|data| data.get_temp::<[f32; 3]>(egui::Id::unique(COLUMN_WIDTHS_ID)))
        .map(|widths| widths.iter().sum::<f32>())
        .unwrap_or_else(|| ui.available_width())
}

/// The columns for the selected player, under `info_strip`'s title: the
/// three fixed columns side by side, `gap` apart, each a `ui.vertical` of
/// panels 14 points apart.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView, gap: f32) {
    let Some((team_id, slot, player_id)) = selected_player(view) else {
        return;
    };
    // Live AATF's held check while it is the selected player's team's:
    // the field crosses below read it (the `live` borrow is disjoint
    // from the session's `&mut` taken next).
    let marks = live_marks::held_violations(view.live_aatf, &view.live.check)
        .filter(|(id, _)| *id == team_id)
        .map(|(id, violations)| live_marks::Marks {
            violations,
            slot,
            team_id: id,
        });
    let (version, positions, stats, mut skills) = {
        let Some(session) = view.session.as_ref() else {
            return;
        };
        let file = session.file();
        let player = file.player(player_id).expect("the selected player");
        (
            file.version(),
            player.positions.clone(),
            player.stats.clone(),
            player.skills.clone(),
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
        ..
    } = view;
    let Some(session) = session.as_mut() else {
        return;
    };
    let generation = *generation;
    let mut skills_changed = false;
    ui.push_id(("player", generation, player_id), |ui| {
        // A row with tops aligned: `horizontal` centers on the cross axis,
        // the columns want their tops level.
        let spacing = ui.spacing().item_spacing;
        // Each column's allocated width is its last pass's measured one
        // (the constants when none is stored), the widths the centring pad
        // reads too.
        let measured: Option<[f32; 3]> = ui
            .ctx()
            .data(|data| data.get_temp(egui::Id::unique(COLUMN_WIDTHS_ID)));
        let saved = measured.unwrap_or([first_column_width(), cards::TWO_COLUMN_WIDTH, 170.0]);
        ui.scope_builder(
            egui::UiBuilder::new().layout(egui::Layout::left_to_right(egui::Align::Min)),
            |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                // Column 1: General info, Other stats, Playable positions —
                // its panels `PANEL_GAP` apart.
                let col1 = ui.allocate_ui_with_layout(
                    egui::vec2(saved[0], ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        // At least the draft's 350 (`.pcol.first`): the
                        // measured width is the next frame's, and content
                        // narrower than 350 would shrink the column to it,
                        // moving the positions grid off the column's centre.
                        ui.set_min_width(first_column_width());
                        ui.spacing_mut().item_spacing = egui::vec2(spacing.x, PANEL_GAP);
                        panel(ui, spacing, "General info", None, 0.0, |ui| {
                            inset(ui, |ui| {
                                info_strip::general(ui, session, team_id, slot, player_id, marks);
                            });
                        });
                        panel(ui, spacing, "Other stats", None, 0.0, |ui| {
                            inset(ui, |ui| {
                                info_strip::other_stats(ui, session, player_id, &stats, marks);
                            });
                        });
                        panel(
                            ui,
                            spacing,
                            PLAYABLE_POSITIONS_LABEL,
                            marks.map(|marks| (marks, live_marks::MarkedField::PlayablePositions)),
                            centered_text_inset(ui),
                            |ui| {
                                playable_positions(ui, session, player_id, &positions);
                            },
                        );
                    },
                );
                // Column 2: Skills (the Cards left badge in its header
                // row), then the ruleset's apart cards in their own
                // panel when it names them, then COM playing styles,
                // their two columns 158 wide.
                let col2 = ui.allocate_ui_with_layout(
                    egui::vec2(saved[1], ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        // The cards the ruleset keeps apart.
                        let aside = rules
                            .as_ref()
                            .and_then(|active| aside_cards(active.rules.ruleset()));
                        ui.spacing_mut().item_spacing = egui::vec2(spacing.x, PANEL_GAP);
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
                        if let Some((title, aside)) = aside.as_ref() {
                            panel(
                                ui,
                                spacing,
                                title,
                                marks.map(|marks| (marks, live_marks::MarkedField::AsideCards)),
                                card_box_inset(ui),
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
                            );
                        }
                        panel(
                            ui,
                            spacing,
                            "COM playing styles",
                            marks.map(|marks| (marks, live_marks::MarkedField::ComStyles)),
                            card_box_inset(ui),
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
                        );
                    },
                );
                // Column 3: the ability stats, one pair a row.
                let col3 = ui.allocate_ui_with_layout(
                    egui::vec2(saved[2], ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(spacing.x, PANEL_GAP);
                        panel(ui, spacing, "Ability stats", None, 0.0, |ui| {
                            // The cells' padding before each label and
                            // after each box, so the column is the draft's.
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(CELL_PADDING, 0))
                                .show(ui, |ui| {
                                    ability_stats(ui, session, player_id, &stats, marks);
                                });
                        });
                    },
                );
                let widths = [
                    col1.response.rect.width(),
                    col2.response.rect.width(),
                    col3.response.rect.width(),
                ];
                ui.ctx().data_mut(|data| {
                    data.insert_temp(egui::Id::unique(COLUMN_WIDTHS_ID), widths);
                });
                // This pass was centred and sized on other widths (none
                // yet, or a new grid's narrow sizing pass): lay it out again
                // on these, so a player's first frame is drawn where every
                // later one is. The constants cannot stand in: column 3's
                // measured width is not its 170.
                if measured != Some(widths) {
                    ui.ctx()
                        .request_discard("the player view's column widths changed");
                }
            },
        );
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

/// One panel of a column: the `ui.strong` title `TITLE_GAP` above the
/// first thing `add` paints — `first_inset` is how deep into `add`'s
/// first row that thing sits (a card row's painted square is centred in
/// the taller control, `card_box_inset`; a field's frame or a label's
/// text paints from the row's top). The title's row is top-aligned like
/// the Skills header's `horizontal_wrapped`, so every title of the view
/// shares the one line — `ui.horizontal`'s row would centre it in the
/// `interact_size` height and sit it lower. One item of the column's
/// vertical — the column's `PANEL_GAP` sits between panels, `spacing`
/// (the pane's own) restored inside. `mark` is Live AATF's held check's
/// with the panel's field: its mark stands before the header.
fn panel(
    ui: &mut egui::Ui,
    spacing: egui::Vec2,
    title: &str,
    mark: Option<(live_marks::Marks<'_>, live_marks::MarkedField)>,
    first_inset: f32,
    add: impl FnOnce(&mut egui::Ui),
) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = spacing;
        ui.scope_builder(
            egui::UiBuilder::new().layout(egui::Layout::left_to_right(egui::Align::Min)),
            |ui| {
                let title = ui.strong(title);
                if let Some((marks, field)) = mark {
                    marks.paint(ui, title.rect, field);
                }
            },
        );
        ui.add_space(TITLE_GAP - spacing.y - first_inset);
        add(ui);
    });
}

/// `add`'s content `CELL_PADDING` in from the panel's title.
fn inset(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: CELL_PADDING,
            ..egui::Margin::ZERO
        })
        .show(ui, add);
}

/// How deep into a `cards::card_columns` row its painted square sits
/// (egui centres the `icon_width` box in the control's `interact_size`
/// height): a card panel's title leaves it that much less of `TITLE_GAP`
/// than a first row painted from its top takes.
fn card_box_inset(ui: &egui::Ui) -> f32 {
    (ui.spacing().interact_size.y - ui.spacing().icon_width) / 2.0
}

/// How deep into a `ui.horizontal` row a line of text sits (egui centres
/// the row's content in the `interact_size` height): the positions'
/// labels' inset — their `first_inset`.
fn centered_text_inset(ui: &egui::Ui) -> f32 {
    (ui.spacing().interact_size.y - ui.text_style_height(&egui::TextStyle::Body)) / 2.0
}

/// `end_row` after `index` pairs (1-based) of `per_row` are done, so a row
/// takes as many pairs as `per_row` allows.
pub(crate) fn maybe_end_row(ui: &mut egui::Ui, index: usize, per_row: usize) {
    if index.is_multiple_of(per_row) {
        ui.end_row();
    }
}

/// One small stored field of the Other stats panel or the Appearance
/// section's Cutscenes group: the term's name, the drawn label, its schema
/// field, the shown range and the shift between shown and stored (the
/// shifted fields are stored one below the game's numbers, the draft's
/// `+ 1`/`v - 1`).
#[derive(Clone, Copy)]
pub(super) struct SmallStat {
    /// The diff row's name for the field (`terms::row_term`); the control
    /// draws `shown`.
    pub(super) label: &'static str,
    /// The control's drawn label (the weak foot's two share one's:
    /// "Weak foot: Usage", then "Accuracy").
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

/// The Other stats panel's four stored fields, in the draft's order: form
/// 1-8, injury resistance 1-3, weak foot usage and accuracy 1-4 (each
/// stored one lower). Star and playing attitude, which change only
/// cutscenes, sit in the Appearance section's `CUTSCENE_STATS`.
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
        label: "Injury resistance",
        shown: "Injury resistance",
        field: PlayerField::InjuryResistance,
        min: 1,
        max: 3,
        shift: 1,
        get: |stats| Some(stats.injury_resistance),
        set: |stats, value| stats.injury_resistance = value,
    },
    SmallStat {
        label: "Weak foot usage",
        shown: "Weak foot: Usage",
        field: PlayerField::WeakFootUsage,
        min: 1,
        max: 4,
        shift: 1,
        get: |stats| Some(stats.weak_foot_usage),
        set: |stats, value| stats.weak_foot_usage = value,
    },
    SmallStat {
        label: "Weak foot accuracy",
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
/// keeps its cell). The Appearance section's Cutscenes group uses it; the
/// Other stats panel's `small` pairs draw the halves apart.
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
            ui.label(small.shown);
            gated_number(ui, small.field);
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

/// The positions grid's C rating's fill (the draft's `.sel.on.rc`
/// `#5a5a5a`): the quiet grey a chosen rating or side gets where the
/// accent would pull the eye — the side toggles' held fill too
/// (`info_strip`'s `side_row`).
const RATING_C_FILL: egui::Color32 = egui::Color32::from_rgb(0x5a, 0x5a, 0x5a);

/// `RATING_C_FILL` on the light theme: the same contrast with the panel,
/// 2.5, as the dark grey has with the dark one, and the light theme's dark
/// text reads on it at 3.05 (on the dark grey itself, at 1.2).
const RATING_C_FILL_LIGHT: egui::Color32 = egui::Color32::from_gray(0x9f);

/// A chosen C rating's fill, and a held side toggle's, on the theme
/// `visuals` draw.
pub(super) fn rating_c_fill(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        RATING_C_FILL
    } else {
        RATING_C_FILL_LIGHT
    }
}

/// The positions grid's A rating's fill (the draft's `.sel.on.ra`
/// `#129648`): a green deep enough to carry a white letter, where the
/// theme's bright success green needed a dark one.
const RATING_A_FILL: egui::Color32 = egui::Color32::from_rgb(0x12, 0x96, 0x48);

/// The chosen rating's fill and text colours (the draft's `.sel.on`
/// variants): A a deep green under a white letter, B the selection colour
/// with its foreground, C a grey square with the normal text — named
/// outright, since a selected button's own text colour is the
/// selection's foreground.
fn rating_colors(ui: &egui::Ui, rating: u8) -> (egui::Color32, egui::Color32) {
    match rating {
        2 => (RATING_A_FILL, egui::Color32::WHITE),
        1 => (
            ui.visuals().selection.bg_fill,
            ui.visuals().selection.stroke.color,
        ),
        _ => (rating_c_fill(ui.visuals()), ui.visuals().text_color()),
    }
}

/// The 13 positions in `POSITION_ROWS` (the rows the Tactics pitch's
/// formation name reads too), each row centred in the column (the
/// draft's `#pos-grid .posrow` `justify-content: center`), each cell the
/// position's label above the three `C` `B` `A` ratings (stored 0, 1, 2;
/// a stored 3 selects none), the cell's contents centred so the label
/// sits over the B, the registered position's cell outlined in the
/// selection colour (the draft's `.poscell.reg`). `playable[i]` is
/// `POSITIONS[PLAYABLE_POSITIONS[i]]`'s rating: the label's index is not
/// the array's.
fn playable_positions(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    positions: &PlayerPositions,
) {
    let mut playable = positions.playable;
    let cell_gap = ui.spacing().item_spacing.x;
    let body = egui::TextStyle::Body.resolve(ui.style());
    // The rows centre in the column: the nominal 350, or the fields'
    // flush edge and their inset where egui's text runs past it.
    let column_width = (info_strip::GENERAL_WIDTH.max(info_strip::general_content_width(ui))
        + f32::from(CELL_PADDING))
    .min(ui.available_width());
    for row in POSITION_ROWS {
        // `add_space` advances the cursor outright, no item spacing.
        let row_width = row.len() as f32 * POS_CELL_WIDTH + (row.len() - 1) as f32 * cell_gap;
        ui.horizontal(|ui| {
            ui.add_space(((column_width - row_width) / 2.0).max(0.0));
            for name in row {
                let index = POSITIONS
                    .iter()
                    .position(|position| position == name)
                    .expect("a POSITION_ROWS name is a POSITIONS name");
                let at = PLAYABLE_POSITIONS
                    .iter()
                    .position(|&position| position == index)
                    .expect("PLAYABLE_POSITIONS is a permutation of 0..13");
                let cell = ui.vertical(|ui| {
                    // The draft's `.poscell` 62px and `.rate`'s squares
                    // 2 apart: `Button::selectable` is what
                    // `selectable_value` draws.
                    ui.set_width(POS_CELL_WIDTH);
                    // The cell's contents centred (the draft's `align-items:
                    // center`): the label padded to the cell's centre, so
                    // over the B; the squares 3.5 in.
                    let galley = ui.painter().layout_no_wrap(
                        (*name).to_owned(),
                        body.clone(),
                        egui::Color32::WHITE,
                    );
                    ui.horizontal(|ui| {
                        ui.add_space(((POS_CELL_WIDTH - galley.size().x) / 2.0).max(0.0));
                        // A label's click makes its position the registered
                        // one (the draft's `.plabel`), the same edit as
                        // General info's Registered position combo; on the
                        // registered position's own label it writes nothing,
                        // so no empty undo entry.
                        let response = ui
                            .add(
                                egui::Label::new(*name)
                                    .sense(egui::Sense::click())
                                    .selectable(false),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(format!("Make {name} the registered position"));
                        if response.clicked() && index != usize::from(positions.registered) {
                            edit_player(session, player_id, response.id.value(), |player| {
                                player.positions.registered =
                                    u8::try_from(index).expect("a POSITIONS index is a byte");
                            });
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = RATING_GAP;
                        ui.spacing_mut().button_padding = egui::vec2(0.0, 0.0);
                        ui.add_space((POS_CELL_WIDTH - RATINGS_WIDTH) / 2.0);
                        for (label, rating) in RATING_LABELS {
                            let response = if playable[at] == rating {
                                // The chosen rating filled (the draft's
                                // `.sel.on.*`).
                                let (fill, text) = rating_colors(ui, rating);
                                ui.add(
                                    egui::Button::selectable(
                                        true,
                                        egui::RichText::new(label).color(text),
                                    )
                                    .min_size(egui::vec2(RATING_SIDE, RATING_SIDE))
                                    .fill(fill),
                                )
                            } else {
                                ui.add(
                                    egui::Button::selectable(false, label)
                                        .min_size(egui::vec2(RATING_SIDE, RATING_SIDE)),
                                )
                            };
                            if response.clicked() && playable[at] != rating {
                                playable[at] = rating;
                                edit_player(session, player_id, response.id.value(), |player| {
                                    player.positions.playable = playable;
                                });
                            }
                        }
                    });
                });
                if index == usize::from(positions.registered) {
                    ui.painter().rect_stroke(
                        cell.response.rect.expand(2.0),
                        ui.visuals().widgets.inactive.corner_radius,
                        egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Outside,
                    );
                }
            }
        });
    }
}

/// The 25 stats of `ABILITY_STATS` in one column in the game's order (the
/// view's third column), each label right-aligned against its `DragValue`
/// 40-99 (the draft's `td.ilabel`), so a short label stands as close to
/// its number as a long one; a stat the version lacks shows `-`, disabled.
/// The label column is measured, the widest label's width, because a
/// right-aligned label needs the column's width before the first row
/// draws. `marks` is Live AATF's held check's: a stat a violation is
/// about carries its mark before the label column's left edge, over the
/// cell padding before it, keyed to the column rather than the text so
/// the stats' marks share one x whatever the labels' lengths.
fn ability_stats(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    stats: &PlayerStats,
    marks: Option<live_marks::Marks<'_>>,
) {
    let abilities = stats.abilities();
    let spacing_y = ui.spacing().item_spacing.y;
    let label_column = ABILITY_STATS
        .iter()
        .map(|label| info_strip::label_width(ui, &title_case(label)))
        .fold(0.0, f32::max);
    let label_height = ui.text_style_height(&egui::TextStyle::Body);
    egui::Grid::new("abilities stats")
        .num_columns(2)
        .spacing(egui::vec2(info_strip::FIELD_ROW_GAP, spacing_y))
        .show(ui, |ui| {
            for (index, &label) in ABILITY_STATS.iter().enumerate() {
                let label_cell = ui
                    .allocate_ui_with_layout(
                        egui::vec2(label_column, label_height),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.set_min_width(label_column);
                            ui.label(title_case(label));
                        },
                    )
                    .response
                    .rect;
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
                        gated_number(ui, ABILITY_FIELDS[index]);
                    }
                }
                ui.end_row();
            }
        });
}

/// The Skills panel's content: the header row's Cards left badge — the
/// player's own tier's, the tier `Rules::player_tier` reads him as, then
/// that tier's `card_limits`; no tier matching draws the stand-in, and a
/// check that fails on him draws the red click line that opens the
/// error in `refusal`, then the checkboxes in their two columns, the
/// skills but the `aside` indices (the aside panel's own).
/// Returns whether a card was ticked this frame. The computed badge
/// lives in `cards` under the `BadgeKey`
/// it was computed for — a `Session::revision` bump, a selection change
/// or another ruleset recomputes it, and any other frame draws the
/// kept one. `marks` is Live AATF's held check's: the header's mark,
/// the skills' and the card budget's in one, stands before the title.
fn skills_panel(
    ui: &mut egui::Ui,
    (rules, aside): (Option<&ActiveRules>, Option<&[u8]>),
    badge: &mut Option<(BadgeKey, OwnBadge)>,
    session: &mut Session,
    (team_id, slot, marks): (u32, u8, Option<live_marks::Marks<'_>>),
    skills: &mut PlayerSkills,
    refusal: &mut Option<Refusal>,
) -> bool {
    if let Some(active) = rules {
        let key = BadgeKey {
            revision: session.revision(),
            team: team_id,
            slot,
            ruleset: active.key.clone(),
        };
        if badge.as_ref().is_none_or(|(held, _)| *held != key) {
            *badge = Some((key, own_badge(&active.rules, session.file(), team_id, slot)));
        }
    }
    // A tier check that fails on him draws `Cards left` then a red click
    // line in the badge's place, opening its error in the refusal dialog
    // (the AATF check never runs the two functions the badge asks, so a
    // member's edit that breaks only them would otherwise show nowhere).
    let tier_error = match badge.as_ref() {
        Some((_, OwnBadge::Failed(error))) => Some(error.clone()),
        _ => None,
    };
    ui.horizontal_wrapped(|ui| {
        let title = ui.strong("Skills");
        if let Some(marks) = marks {
            marks.paint(ui, title.rect, live_marks::MarkedField::Skills);
        }
        if let Some(error) = &tier_error {
            ui.weak("Cards left:");
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
            match badge.as_ref() {
                Some((_, OwnBadge::Tier { label, limits })) => cards::cards_left(
                    ui,
                    &cards::CardsBadge::Tier(cards::TierCards { label, limits }),
                ),
                Some((_, OwnBadge::NoTier)) => cards::cards_left(ui, &cards::CardsBadge::NoTier),
                Some((_, OwnBadge::Failed(_))) | None => {
                    ui.weak("Cards left:");
                }
            }
        }
    });
    // The title stands `TITLE_GAP` above the first card row's painted
    // square, as `panel`'s do.
    ui.add_space(TITLE_GAP - ui.spacing().item_spacing.y - card_box_inset(ui));
    let shown: Vec<u8> = all_indices(&labels::SKILLS)
        .into_iter()
        .filter(|index| !aside.is_some_and(|aside| aside.contains(index)))
        .collect();
    cards::card_columns(
        ui,
        session.file().version(),
        &mut skills.skills,
        &labels::SKILLS,
        &shown,
        PlayerField::Skill,
    )
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

/// What `rules` read team `team_id`'s roster `slot` as on `file`: his own
/// tier with its card economy, no tier, or a check that failed with its
/// error (a team the view cannot build, or an empty slot, fails too).
fn own_badge(rules: &aatf::Rules, file: &EditFile, team_id: u32, slot: u8) -> OwnBadge {
    let team = match TeamView::new(file, team_id) {
        Ok(team) => team,
        Err(error) => return OwnBadge::Failed(error.to_string()),
    };
    let Some(player) = team.player(slot) else {
        return OwnBadge::Failed(format!("roster slot {slot} holds no player"));
    };
    match rules.player_tier(&team, player) {
        Ok(Some(tier)) => match rules.card_limits(&team, player, &tier.id) {
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
    let mut response = ui.add(
        egui::DragValue::new(&mut shown)
            .range(range.clone())
            .clamp_existing_to_range(false),
    );
    if step_number(&mut shown, wheel_steps(ui, &response), range) {
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
/// SE-GUI-06) with the `PES NN+` tooltip.
pub(crate) fn gated_number(ui: &mut egui::Ui, field: PlayerField) {
    ui.add_enabled(
        false,
        egui::Button::new("-").min_size(ui.spacing().interact_size),
    )
    .on_disabled_hover_text(gating_tooltip(field));
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
mod tests {
    use pes_savefile::schema::fields::ABILITY_FIELDS;
    use pes_savefile::schema::fields::PlayerField;
    use pes_savefile::schema::fields::PlayerText;
    use pes_savefile::schema::{RecordSchema, schema_for};
    use pes_version::PesVersion;

    use super::{SMALL_STATS, aside_cards};

    /// The `2^bit_width - 1` bound `info_strip.rs`'s `max` reads, over the
    /// appearance record too (a field may live there on PES 15/16); `None`
    /// on a version that does not store `field`.
    fn field_max(version: PesVersion, field: PlayerField) -> Option<u64> {
        fn in_record(
            record: &RecordSchema<PlayerField, PlayerText>,
            field: PlayerField,
        ) -> Option<u64> {
            record
                .fields
                .iter()
                .find(|spec| spec.field == field)
                .map(|spec| u64::from(spec.bit_width))
                .or_else(|| {
                    record.arrays.iter().find_map(|array| {
                        (0..array.count)
                            .any(|i| (array.make)(i) == field)
                            .then_some(u64::from(array.bit_width))
                    })
                })
        }
        let schema = schema_for(version);
        let width = in_record(schema.player, field).or_else(|| {
            schema
                .appearance
                .as_ref()
                .and_then(|(_, record)| in_record(record, field))
        })?;
        Some((1u64 << width) - 1)
    }

    /// `name`'s index in `labels::SKILLS` (a card's index is its
    /// position there).
    fn skill(name: &str) -> u8 {
        u8::try_from(
            pes_savefile::interchange::team_toml::labels::SKILLS
                .iter()
                .position(|&label| label == name)
                .expect("a skill"),
        )
        .expect("a byte index")
    }

    /// The smallest ruleset the crate's loader accepts, holding `body`
    /// (the `card_groups`/`specials` lines the case varies): the check
    /// asks the tiers' counts for the squad, so a rest tier is always
    /// there.
    fn ruleset(body: &str) -> aatf::Ruleset {
        let block = format!(
            "const RULESET = #{{
                schema: 1, name: \"x\", squad_size: 23,
                universal: #{{ max_skill_cards: 10 }},
                tiers: [#{{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                            injury_resistance: 1, weak_foot: #{{ usage: 2, accuracy: 2 }},
                            cards: 3, a_positions: 1 }}],
                {body}
            }};"
        );
        aatf::Ruleset::from_data_block(&block).expect("the block loads")
    }

    // The aside panel's set: a `tricks` group names it, else the skills
    // the `free_cards` effects name — a free card inside a `choice`
    // option counts — in `SKILLS` order with none twice.
    #[test]
    fn the_aside_holds_the_tricks_else_the_free_skills() {
        // A `tricks` group: its skills in SKILLS order however the file
        // lists them (its `free_cards` specials are not read then).
        let rs = ruleset(
            "card_groups: #{ tricks: [\"malicia\", \"scissors_feint\"] },
             specials: [#{ label: \"s\", then: #{ free_cards: [\"heading\"] } }],",
        );
        let (title, indices) = aside_cards(&rs).expect("the group names cards");
        assert_eq!(title, "Tricks");
        assert_eq!(indices, [skill("scissors_feint"), skill("malicia")]);

        // No `tricks` group: every skill a `free_cards` effect names,
        // a conditional special's and a choice option's included, the
        // COM styles left out.
        let rs = ruleset(
            "specials: [
                #{ label: \"free\", then: #{ free_cards: [\"malicia\", \"long_ranger\"] } },
                #{ label: \"conditional\", when: #{ captain: true },
                   then: #{ free_cards: [\"captaincy\"] } },
                #{ label: \"pick\",
                   then: #{ choice: [#{ free_cards: [\"heading\"] }, #{ cards: 1 }] } },
            ],",
        );
        let (title, indices) = aside_cards(&rs).expect("the free cards name skills");
        assert_eq!(title, "Free cards");
        assert_eq!(
            indices,
            [skill("heading"), skill("malicia"), skill("captaincy")],
            "the choice's, the conditional's and the plain effect's skills, in SKILLS order"
        );

        // A ruleset whose only free cards sit in a choice's options:
        // they count too.
        let rs = ruleset(
            "specials: [#{ label: \"pick\",
                then: #{ choice: [#{ free_cards: [\"heading\"] }, #{ cards: 1 }] } }],",
        );
        assert_eq!(
            aside_cards(&rs),
            Some(("Free cards", vec![skill("heading")]))
        );

        // Neither: no aside panel.
        assert_eq!(aside_cards(&ruleset("")), None);
    }

    // Every range's top fits the field's bit width on every version
    // storing it (after the stored-one-lower shift). The Appearance
    // section's Cutscenes pair is a `SmallStat` too.
    #[test]
    fn every_ranges_top_fits_the_fields_bit_width() {
        for (field, top, shift) in SMALL_STATS
            .iter()
            .chain(crate::view::appearance::fields::CUTSCENE_STATS.iter())
            .map(|small| (small.field, small.max, small.shift))
            .chain(ABILITY_FIELDS.iter().map(|&field| (field, 99u8, 0u8)))
        {
            for version in PesVersion::ALL {
                let Some(max) = field_max(version, field) else {
                    continue;
                };
                assert!(
                    u64::from(top - shift) <= max,
                    "{version:?} {field:?}: shown top {top} stores {} past {max}",
                    top - shift
                );
            }
        }
    }
}
