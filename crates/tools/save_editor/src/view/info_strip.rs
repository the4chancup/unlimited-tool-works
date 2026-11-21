//! The player view's title and the first column's first two panels (the
//! draft's `playerHeader` and `generalPanel`): `<id> <name>` at the
//! heading size, then General info — Name and Shirt name, each a
//! label-left row carrying its `(n/max)` count beside its field, the
//! Shirt number | Age pair on two halves (each box 54 wide: on the
//! first half's right edge, after the label column on the second), then
//! the Nation combo — and Main stats under it, Captain in its title's
//! row, the pair rows on the same halves so every box of both panels
//! lines up, then the combos whose labels share one column. Each
//! control edits a copy of the entry and, when it changes, pushes it
//! under its own `egui::Id` (the undo stack's merge key).

use std::borrow::Cow;

use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::names::{crop_player_text, name_fits, shirt_name_edit_fits};
use pes_savefile::model::{PlayerEntry, PlayerPositions, PlayerStats};
use pes_savefile::schema::fields::{PlayerField, PlayerText, RosterField};
use pes_savefile::schema::{VersionSchema, portable_text_max, schema_for, stored_offset};
use pes_version::PesVersion;
use studio_core::widgets::{
    FIELD_SPACING, RADIO_ICON_WIDTH, add_enabled_ui, combo_item, number_arrows, number_wheel_steps,
    radio, resting_edge, step_choice, step_number, text_box, wheel_steps,
};
use team_widgets::cards;
use team_widgets::names::append_name;

use crate::nations::nations;
use crate::session::Session;
use crate::session::undo::RUN_PAUSE;
use crate::view::abilities::{
    CELL_PADDING, PLAYING_STYLE_LABEL, REGISTERED_POSITION_LABEL, SMALL_STATS, STRONGER_FOOT_LABEL,
    STRONGER_HAND_LABEL, SmallStat, gated_label, gating_tooltip, position_label, small_stat_max,
};
use crate::view::live_marks::{MarkedField, Marks};
use crate::view::{
    PlayerScreen, Reveal, SaveView, Selection, edit_player, edit_team, selected_player,
    selected_team_id,
};

/// The General info panel's Name field.
pub(super) const NAME_LABEL: &str = "Name";
/// The General info panel's Shirt name field.
pub(super) const SHIRT_LABEL: &str = "Shirt name";
/// The Main stats panel's Height field.
pub(super) const HEIGHT_LABEL: &str = "Height";
/// The Main stats panel's Weight field.
pub(super) const WEIGHT_LABEL: &str = "Weight";
/// The General info panel's Age field.
pub(super) const AGE_LABEL: &str = "Age";
/// The General info panel's Nation field.
pub(super) const NATION_LABEL: &str = "Nation";
/// The General info panel's Shirt number field.
pub(super) const NUMBER_LABEL: &str = "Shirt number";
/// The Main stats panel's Captain field, in its title's row.
pub(super) const CAPTAIN_LABEL: &str = "Captain";

/// General info's fields' width, the panel's nominal width in the draft
/// (344px): with the content's inset it is the first column's floor.
pub(super) const GENERAL_WIDTH: f32 = 344.0;
/// A pair's box width (the draft's `.pair` fields).
const BOX_WIDTH: f32 = 54.0;
/// The gap between a label and its field (the draft's `.pair` `gap` and
/// `.combos` `column-gap`, 20 points), the same everywhere in the panels:
/// on the label-left rows, on both of a pair's halves and in Ability
/// stats.
pub(super) const LABEL_GAP: f32 = 20.0;
/// The least room between a pair row's two halves (the draft's `.pairs`
/// `column-gap`, 18 points); the second half ends on the panel's flush
/// edge and the halves take what room is between them.
const MIN_HALF_GAP: f32 = 18.0;

/// A player name as the strip draws it: its `\x11` spelled as the four
/// characters `\\x11` (the draft's `escName`); the field turns it back on
/// edit.
pub(super) fn shown_name(name: &str) -> String {
    name.replace('\x11', "\\x11")
}

/// The title's row for the selected player; nothing when the selection
/// names no rostered player (a stale slot, an empty save's list).
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some((_, _, player_id)) = selected_player(view) else {
        return;
    };
    let Some(session) = view.session.as_ref() else {
        return;
    };
    let name = session
        .file()
        .player(player_id)
        .expect("the selected player")
        .name
        .clone();
    view_title(ui, view, player_id, &name);
}

/// A view's title's row, the player's screens' and the team view's alike
/// (the draft's `viewTitle`): the three screens' toggle (`page_toggle`),
/// then `<id> <name>` — the id in the weak colour as the list draws ids,
/// the name's colour codes drawn as the list draws them (`append_name`),
/// its plain runs in the strong colour, all in the body text's size (the
/// app installs no bold face) — and the separator under the row (the
/// `.sep` the fields' row carried), which parts the title from the screen
/// the toggle switches.
pub(super) fn view_title(ui: &mut egui::Ui, view: &mut SaveView, id: u32, name: &str) {
    // The row's room over it is the pane's `TITLE_MARGIN`, the panes' own
    // start and this half point — 9.5, the same as its room under it to
    // the separator (the draft's `.pheader` 5.75 over, `.sep` 9.25 under).
    ui.add_space(0.5);
    // The draft's `.pheader` is text-high like the `.sel`s in it;
    // `ui.horizontal` allocates its row a control's `interact_size` tall,
    // which would stand the separator 3 points lower.
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = ui.text_style_height(&egui::TextStyle::Body);
        ui.horizontal(|ui| {
            page_toggle(ui, view);
            let body = egui::TextStyle::Body.resolve(ui.style());
            let mut title = egui::text::LayoutJob::default();
            title.append(
                &format!("{id} "),
                0.0,
                egui::TextFormat::simple(body.clone(), ui.visuals().weak_text_color()),
            );
            append_name(&mut title, name, &body);
            let strong = ui.visuals().strong_text_color();
            for section in &mut title.sections {
                if section.format.color == egui::Color32::PLACEHOLDER {
                    section.format.color = strong;
                }
            }
            ui.label(title);
        });
    });
    // The `.sep` under the row, its centre 9.5 under it: this 2.5 plus
    // `SEPARATOR_MARGIN` over the painted row (the draft's 9.25), and
    // `SEPARATOR_MARGIN` under it — 11.5 over the first frames' tops.
    ui.add_space(2.5);
    let trailing = ui.spacing().item_spacing.y;
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke.width;
    ui.add(egui::Separator::default().spacing(2.0 * (SEPARATOR_MARGIN - trailing) + stroke));
}

/// The `.sep`'s margin under its painted row (the draft's margin-bottom):
/// its centre then stands 11.5 over the first frames' tops.
const SEPARATOR_MARGIN: f32 = 7.0;

/// One segment of the title's toggle: the two player screens and the
/// team view.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Segment {
    Stats,
    Appearance,
    Gameplan,
}

impl Segment {
    /// The segment's text.
    fn label(self) -> &'static str {
        match self {
            Segment::Stats => "Stats",
            Segment::Appearance => "Appearance",
            Segment::Gameplan => "Gameplan",
        }
    }
}

/// The three screens' toggle at the title row's left (the draft's
/// `pageToggle`): `Stats`, `Appearance` and `Gameplan` drawn as the
/// tactics' numbers toggles draw their three choices (a point apart, the
/// `.sel` 5-point padding, the held one filled in the selection colour).
/// `Gameplan` is held while a team row is selected, else the player
/// screen shown (`view.page`). A press on another segment moves the
/// screen and, as a row's click does, the selection: `Gameplan` selects
/// the selected row's team's row; a player screen from a team row
/// selects the team's remembered or first filled slot — the list
/// scrolling to show the row (`reveal_selected`). Not drawn with nothing
/// selected.
fn page_toggle(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some(team_id) = selected_team_id(view) else {
        return;
    };
    let on = match view.selected {
        Some(Selection::Team(_)) => Segment::Gameplan,
        Some(Selection::Player(..)) | None => match view.page {
            PlayerScreen::Stats => Segment::Stats,
            PlayerScreen::Appearance => Segment::Appearance,
        },
    };
    let pressed = ui
        .scope(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            ui.spacing_mut().button_padding = egui::vec2(5.0, 0.0);
            // The draft's `.sel` is text-high, where a button stands a
            // control's `interact_size` tall.
            ui.spacing_mut().interact_size.y = ui.text_style_height(&egui::TextStyle::Body);
            // Every segment is drawn before the press is acted on: stopping
            // at the pressed one left the segments after it undrawn for a
            // frame, and the title stood beside it.
            let held_edge =
                studio_core::theme::edge_color(ui.visuals(), ui.visuals().selection.bg_fill);
            let unheld = studio_core::theme::unheld_edge(ui.visuals());
            let mut pressed = None;
            for segment in [Segment::Stats, Segment::Appearance, Segment::Gameplan] {
                let response = ui.add(egui::Button::selectable(on == segment, segment.label()));
                resting_edge(
                    ui,
                    &response,
                    if on == segment { held_edge } else { unheld },
                );
                if response.clicked() {
                    pressed = Some(segment);
                }
            }
            pressed
        })
        .inner;
    let Some(segment) = pressed else {
        return;
    };
    if segment == on {
        return;
    }
    match segment {
        Segment::Gameplan => {
            view.selected = Some(Selection::Team(team_id));
            view.reveal_selected = Some(Reveal::Nearest);
        }
        Segment::Stats | Segment::Appearance => {
            view.page = match segment {
                Segment::Stats => PlayerScreen::Stats,
                _ => PlayerScreen::Appearance,
            };
            if matches!(view.selected, Some(Selection::Team(_)))
                && let Some(slot) = last_shown_slot(view, team_id)
            {
                view.selected = Some(Selection::Player(team_id, slot));
                view.reveal_selected = Some(Reveal::Nearest);
            }
        }
    }
}

/// The team's roster slot a player screen selects from its row: the
/// player last shown on it since the save opened, the first filled slot
/// when none was shown (or the remembered slot no longer holds one).
fn last_shown_slot(view: &SaveView, team_id: u32) -> Option<u8> {
    let session = view.session.as_ref()?;
    let shown = |slot: u8| session.slot_player(team_id, slot).is_some();
    if let Some(&slot) = view.last_shown.get(&team_id)
        && shown(slot)
    {
        return Some(slot);
    }
    let roster = session.file().team(team_id)?.roster.len();
    (0..roster)
        .map(|slot| u8::try_from(slot).expect("a roster is at most 40 slots"))
        .find(|&slot| shown(slot))
}

/// The General info panel's content (`generalPanel`'s `general`): the
/// Name and Shirt name counted fields, the Shirt number | Age pair row,
/// then the Nation combo. `marks` is Live AATF's held check's, while it
/// is the selected player's team's: each marked field's mark then
/// stands before its label (`Marks::paint`). `focused` is the view's
/// last-frame keyboard holder: a typed Nation pick's row is recorded
/// on it so the frame's gesture-end check reads the pick's focus move
/// as the gesture's own, not the keyboard leaving a widget.
pub(super) fn general(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    slot: u8,
    player_id: u32,
    marks: Option<Marks<'_>>,
    focused: &mut Option<egui::Id>,
) {
    let (mut name, stored_name, mut shirt, stored_shirt, nation, mut shirt_number, age, schema) = {
        let file = session.file();
        let player = file.player(player_id).expect("the selected player");
        let team = file.team(team_id).expect("the selected team");
        (
            // The name field shows the raw name, its `\x11` spelled as the
            // four characters `\\x11` (the draft's `escName`); on edit the
            // field's text is turned back before it is stored.
            shown_name(&player.name),
            player.name.clone(),
            player.shirt_name.clone(),
            player.shirt_name.clone(),
            player.basic.nationality,
            team.roster
                .get(usize::from(slot))
                .map_or(0, |slot| slot.number),
            player.basic.age,
            schema_for(file.version()),
        )
    };

    // The rows' layout (the draft's `.combos` grid): the label column,
    // the 20-point gap, the fields to the flush edge (12 inside the
    // frame's border). The pairs share it: the first half is the label
    // column, the gap and a box, the second its own label column, the
    // gap and a box on the same edge. Content needing more room than the
    // panel's overflows rather than shrinking a gap.
    let label_col = label_width(ui, REGISTERED_POSITION_LABEL);
    let field_x = ui.max_rect().min.x + label_col + LABEL_GAP;
    let flush_x = flush_edge(ui);
    let field_width = flush_x - field_x;

    // Each a label-left row: the `label (n/max)` label's rect taken at
    // the row's left, the label itself painted after the field — the
    // count follows the field's own text this frame (a TextEdit writes
    // its buffer during `ui.add`), not the stored name. The field and
    // its count carry one hover.
    let overlong = stored_name.len() > portable_text_max(PlayerText::Name);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = LABEL_GAP;
        let (name_label_id, name_label) = ui.allocate_space(egui::vec2(
            label_col,
            ui.text_style_height(&egui::TextStyle::Body),
        ));
        let mut name_edit = egui::TextEdit::singleline(&mut name);
        if overlong {
            name_edit = name_edit.background_color(warning_tint(ui.visuals()));
        }
        let response = ui.add_sized(
            [field_width, ui.spacing().interact_size.y],
            text_box(name_edit),
        );
        // A name past what every version holds is drawn tinted with the
        // cut PES 15 to 19 would make: a warning, never a block on Save.
        let tip = if overlong {
            format!(
                "In PES 2019 and older the game cuts this name to «{}»",
                shown_name(&crop_player_text(
                    PesVersion::Pes19,
                    PlayerText::Name,
                    &stored_name
                ))
            )
        } else {
            format!(
                "Up to {} bytes: one for a plain letter, two or more for an accented letter, \
                 and ten for a color code (typed as \\x11c and eight hex digits).",
                text_len(schema, PlayerText::Name)
            )
        };
        counted_label(
            ui,
            name_label_id,
            name_label,
            NAME_LABEL,
            (
                name.replace("\\x11", "\x11").len(),
                text_len(schema, PlayerText::Name),
            ),
            text_len(schema, PlayerText::Name).min(portable_text_max(PlayerText::Name)),
        )
        .on_hover_text(&tip);
        if let Some(marks) = marks {
            marks.paint(ui, name_label, MarkedField::Name);
        }
        let response = response.on_hover_text(&tip);
        if response.changed() {
            let raw = name.replace("\\x11", "\x11");
            // An edit the codec would refuse is not pushed (the save's own
            // field is the bound): the field shows the stored text again
            // next frame.
            if name_fits(schema.version, &raw) {
                edit_player(session, player_id, response.id.value(), |player| {
                    player.name = raw;
                    player.edit_flags.player = true;
                });
            }
        }
    });

    let shirt_overlong = stored_shirt.chars().count() > portable_text_max(PlayerText::ShirtName);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = LABEL_GAP;
        let (shirt_label_id, shirt_label) = ui.allocate_space(egui::vec2(
            label_col,
            ui.text_style_height(&egui::TextStyle::Body),
        ));
        let mut shirt_edit = egui::TextEdit::singleline(&mut shirt);
        if shirt_overlong {
            shirt_edit = shirt_edit.background_color(warning_tint(ui.visuals()));
        }
        let response = ui.add_sized(
            [field_width, ui.spacing().interact_size.y],
            text_box(shirt_edit),
        );
        let tip = if shirt_overlong {
            "PES 2019 and older show 18 letters of a shirt name (PES 2014 and 2016 show 16)"
                .to_owned()
        } else {
            // The limit the field enforces, not the field's whole capacity:
            // the portable 18 binds on PES 20/21, its own 16 on PES 16, and
            // the charset is single-byte (é passes, ł does not).
            format!(
                "Up to {} letters. Western European accents (é, ü, ñ) are fine, but other \
                 letters and symbols are not.",
                portable_text_max(PlayerText::ShirtName)
                    .min(text_len(schema, PlayerText::ShirtName))
            )
        };
        let shirt_limit =
            portable_text_max(PlayerText::ShirtName).min(text_len(schema, PlayerText::ShirtName));
        counted_label(
            ui,
            shirt_label_id,
            shirt_label,
            SHIRT_LABEL,
            (shirt.chars().count(), shirt_limit),
            shirt_limit,
        )
        .on_hover_text(&tip);
        let response = response.on_hover_text(&tip);
        if response.changed() {
            // Upper-cased as typed, then the fit check (the portable limit
            // or the version's own, whichever is shorter; a longer stored
            // name shortens, never lengthens).
            let upper = shirt.to_uppercase();
            if shirt_name_edit_fits(schema.version, &stored_shirt, &upper) {
                edit_player(session, player_id, response.id.value(), |player| {
                    player.shirt_name = upper;
                    player.edit_flags.player = true;
                });
            }
        }
    });

    // The label-left pair on two halves: Shirt number on the first
    // (its `edit_team` roster-number edit), Age on the second.
    pair_row(ui, |ui| {
        pair_half(ui, Half::First, NUMBER_LABEL, None, |ui| {
            let range = 0..=roster_number_max(schema);
            let mut response = number_arrows(ui, &mut shirt_number, range.clone(), |ui, value| {
                ui.add_sized(
                    [BOX_WIDTH, ui.spacing().interact_size.y],
                    studio_core::widgets::number_box(
                        egui::DragValue::new(value).range(range.clone()),
                    ),
                )
            });
            if step_number(&mut shirt_number, number_wheel_steps(ui, &response), range) {
                response.mark_changed();
            }
            if response.changed() {
                edit_team(session, team_id, response.id.value(), |team| {
                    team.roster[usize::from(slot)].number = shirt_number;
                });
            }
        });
        pair_half(
            ui,
            Half::Second,
            AGE_LABEL,
            marks.map(|marks| (marks, MarkedField::Age)),
            |ui| {
                number_box(
                    ui,
                    session,
                    player_id,
                    age,
                    shown_range(schema, PlayerField::Age),
                    |player, shown| player.basic.age = shown,
                );
            },
        );
    });

    // A typed pick's row takes the keyboard (`nation_row` returns its
    // id): the move is the pick's own gesture, recorded here as already
    // held so the frame's gesture-end check does not seal the picks'
    // undo run as a keyboard move off a widget.
    if let Some(row) = nation_row(
        ui,
        session,
        player_id,
        nation,
        schema.version,
        label_col,
        field_width,
    ) {
        *focused = Some(row);
    }
}

/// One row of the pairs' two halves, the room between them at least
/// `MIN_HALF_GAP`.
fn pair_row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        add(ui);
    });
}

/// Which of a pair row's two halves a pair sits on: the first takes the
/// half's own width and puts its box on its right edge, the second —
/// the row's last item — ends on the flush edge, after the room the
/// first half leaves it.
#[derive(Clone, Copy)]
enum Half {
    /// The row's left half (the draft's `.pair` flex row).
    First,
    /// The row's right half (the draft's `.pair:nth-child(even)` grid).
    Second,
}

/// One `label`-left pair of a `pair_row`: the label at the half's left;
/// on `Half::First` `add_box`'s 54-wide box on the half's right edge (the
/// half is the label-left rows' label column, their `LABEL_GAP` and a
/// box wide, so its boxes start at the x those rows' fields start at),
/// on `Half::Second` just after the `second_half_column` label column
/// and its gap, at the row's right end (`MIN_HALF_GAP` or more after
/// the first half), so both panels' second-half boxes share one x.
/// `mark` is the field Live AATF's mark is about on this half, painted
/// before the label (`Marks::paint`).
fn pair_half(
    ui: &mut egui::Ui,
    half: Half,
    label: &str,
    mark: Option<(Marks<'_>, MarkedField)>,
    add_box: impl FnOnce(&mut egui::Ui),
) {
    match half {
        Half::First => {
            let width = label_width(ui, REGISTERED_POSITION_LABEL) + LABEL_GAP + BOX_WIDTH;
            ui.allocate_ui(egui::vec2(width, ui.spacing().interact_size.y), |ui| {
                ui.horizontal(|ui| {
                    let response = ui.label(label);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add_box);
                    if let Some((marks, field)) = mark {
                        marks.paint(ui, response.rect, field);
                    }
                });
            });
        }
        Half::Second => {
            // The draft's `.pair:nth-child(even)` grid at the row's right:
            // the `second_half_column` label column, the gap and the box
            // ending on the flush edge. `add_space` advances the cursor
            // outright, no item spacing.
            let column = second_half_column(ui);
            let lead = (flush_edge(ui) - column - LABEL_GAP - BOX_WIDTH - ui.cursor().min.x)
                .max(MIN_HALF_GAP);
            ui.add_space(lead);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let response = ui.label(label);
                ui.add_space((column + LABEL_GAP - response.rect.width()).max(0.0));
                if let Some((marks, field)) = mark {
                    marks.paint(ui, response.rect, field);
                }
                add_box(ui);
            });
        }
    }
}

/// The second half's label column's width (the draft's `.pair
/// :nth-child(even)` label column): the widest second-half label of
/// both panels, measured as `label_width` measures, so a short label
/// (Age, Form) stands as near its box as the widest allows, every
/// second-half label of both panels at one x.
fn second_half_column(ui: &egui::Ui) -> f32 {
    [
        AGE_LABEL,
        WEIGHT_LABEL,
        SMALL_STATS[0].shown,
        SMALL_STATS[3].shown,
        CAPTAIN_LABEL,
    ]
    .into_iter()
    .map(|label| label_width(ui, label))
    .fold(0.0, f32::max)
}

/// The panels' fields' shared right edge: `CELL_PADDING` in from the
/// panel frame's content edge, so a field ends 12 inside the frame's
/// border (the draft's `.gen` `padding-right` inside the frame's own).
fn flush_edge(ui: &egui::Ui) -> f32 {
    ui.max_rect().max.x - f32::from(CELL_PADDING)
}

/// `label`'s galley width at the body font (the panels' label columns
/// are measured rather than hard-coded: egui's text is wider than the
/// draft's).
pub(super) fn label_width(ui: &egui::Ui, label: &str) -> f32 {
    let body = egui::TextStyle::Body.resolve(ui.style());
    ui.painter()
        .layout_no_wrap(label.to_owned(), body, egui::Color32::PLACEHOLDER)
        .size()
        .x
}

/// One pair's 54-wide `DragValue` over `shown` inside `bounds`, the
/// field's shown range (`shown_range`; an end the widget's own type
/// cannot hold is capped at it). A change pushes the player's entry
/// with `set` applied.
fn number_box<N: egui::emath::Numeric + TryFrom<u64>>(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    shown: N,
    bounds: std::ops::RangeInclusive<u64>,
    set: impl FnOnce(&mut PlayerEntry, N),
) {
    let mut shown = shown;
    let range = N::try_from(*bounds.start()).unwrap_or(N::MIN)
        ..=N::try_from(*bounds.end()).unwrap_or(N::MAX);
    let mut response = number_arrows(ui, &mut shown, range.clone(), |ui, value| {
        ui.add_sized(
            [BOX_WIDTH, ui.spacing().interact_size.y],
            studio_core::widgets::number_box(egui::DragValue::new(value).range(range.clone())),
        )
    });
    if step_number(&mut shown, number_wheel_steps(ui, &response), range) {
        response.mark_changed();
    }
    if response.changed() {
        edit_player(session, player_id, response.id.value(), |player| {
            set(player, shown);
        });
    }
}

/// The Main stats panel's content (`generalPanel`'s `main`): the pair
/// rows on General info's halves, so every box of both panels lines up
/// — Height with Weight, then the four shifted stored fields, Injury
/// Resistance with Form, then "Weak Foot: Usage" and "Accuracy" (the
/// two sharing one label) — then the combos (Stronger foot and hand,
/// Registered position, Playing style) on the label-left rows' column.
/// `marks` is Live AATF's held check's, while it is the selected
/// player's team's.
pub(super) fn main_stats(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    stats: &PlayerStats,
    positions: &PlayerPositions,
    version: PesVersion,
    marks: Option<Marks<'_>>,
) {
    let (height, weight) = {
        let player = session
            .file()
            .player(player_id)
            .expect("the selected player");
        (player.basic.height, player.basic.weight)
    };
    let schema = schema_for(version);
    pair_row(ui, |ui| {
        pair_half(
            ui,
            Half::First,
            HEIGHT_LABEL,
            marks.map(|marks| (marks, MarkedField::Height)),
            |ui| {
                number_box(
                    ui,
                    session,
                    player_id,
                    height,
                    shown_range(schema, PlayerField::Height),
                    |player, shown| player.basic.height = shown,
                );
            },
        );
        pair_half(
            ui,
            Half::Second,
            WEIGHT_LABEL,
            marks.map(|marks| (marks, MarkedField::Weight)),
            |ui| {
                number_box(
                    ui,
                    session,
                    player_id,
                    weight,
                    shown_range(schema, PlayerField::Weight),
                    |player, shown| player.basic.weight = shown,
                );
            },
        );
    });
    let pairs = [
        [
            (&SMALL_STATS[1], MarkedField::InjuryResistance),
            (&SMALL_STATS[0], MarkedField::Form),
        ],
        [
            (&SMALL_STATS[2], MarkedField::WeakFootUsage),
            (&SMALL_STATS[3], MarkedField::WeakFootAccuracy),
        ],
    ];
    for [(left, left_field), (right, right_field)] in pairs {
        pair_row(ui, |ui| {
            pair_half(
                ui,
                Half::First,
                left.shown,
                marks.map(|marks| (marks, left_field)),
                |ui| {
                    small_box(ui, session, player_id, stats, left);
                },
            );
            pair_half(
                ui,
                Half::Second,
                right.shown,
                marks.map(|marks| (marks, right_field)),
                |ui| {
                    small_box(ui, session, player_id, stats, right);
                },
            );
        });
    }

    // The combos, on the same label-left rows as Name's and Shirt
    // name's in General info.
    let label_col = label_width(ui, REGISTERED_POSITION_LABEL);
    let field_x = ui.max_rect().min.x + label_col + LABEL_GAP;
    let field_width = flush_edge(ui) - field_x;
    combos(
        ui,
        session,
        player_id,
        positions,
        version,
        field_width,
        marks,
    );
}

/// How far Captain's circle stands above its row's middle: the draft's
/// radio, the browser's own, stands 1 point high in its box (measured at
/// 1280, its circle's centre at y 219 where the row's middle is 220).
const CAPTAIN_RADIO_RISE: f32 = 1.0;

/// The Captain pair of Main stats' title row (the draft's
/// `.phead.captained`): the label where the second half's labels stand
/// and the radio centred over their boxes — a 54-wide cell ending on
/// the flush edge — drawn through a child `Ui` at `title`'s rect so
/// its 18-point box overhangs the bare title's row rather than growing
/// it (the draft's `margin: -1.5px`), every row under standing where it
/// would under a bare title. `captain` is whether the selected player's
/// slot is the team's captain. Returns the radio's id value — the
/// edit's undo merge key — on a click.
pub(super) fn captain_title(ui: &mut egui::Ui, title: egui::Rect, captain: bool) -> Option<u64> {
    let column = second_half_column(ui);
    let rect = egui::Rect::from_min_max(
        egui::pos2(flush_edge(ui) - BOX_WIDTH - LABEL_GAP - column, title.min.y),
        egui::pos2(flush_edge(ui), title.max.y),
    );
    // A `new_child`, not a `scope`: the child's min rect never reaches
    // the parent's, so the overhanging box leaves the title's row a
    // bare title's height.
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("captain")
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 0.0;
    let label = child.label(CAPTAIN_LABEL);
    child.add_space((column + LABEL_GAP - label.rect.width()).max(0.0));
    let mut clicked = None;
    child.allocate_ui(
        egui::vec2(BOX_WIDTH, child.spacing().interact_size.y),
        |ui| {
            // The circle centred over the boxes' column, as the draft's,
            // and `CAPTAIN_RADIO_RISE` above the row's middle. An
            // unlabelled radio is `interact_size.y` square with its
            // circle at its left, so centring the radio itself would
            // stand the circle left of the column's centre.
            let side = ui.spacing().interact_size.y;
            let rect = egui::Rect::from_center_size(
                ui.max_rect().center()
                    + egui::vec2((side - RADIO_ICON_WIDTH) / 2.0, -CAPTAIN_RADIO_RISE),
                egui::vec2(side, side),
            );
            let response = ui
                .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    radio(ui, captain, "")
                })
                .inner;
            if response.clicked() {
                clicked = Some(response.id.value());
            }
        },
    );
    clicked
}

/// What the open Nation list's typed `run` picks: a run starting with
/// a digit prefixes an ID, a letter run a name — the first nation in
/// `own`'s ID order it prefixes (a `231` run lands on SAN MARINO, `san`
/// too), `None` for a run that prefixes nothing. The candidates are
/// `own`, the version's own list, alone: an unlisted held's `?` row
/// stays clickable, but a pick that landed on it would store nothing,
/// and the real nations its digits or letters start could never be
/// typed. The names are the game's capitals, some accented, so the run
/// goes up in capitals.
fn run_pick(own: &[(u16, String)], run: &str) -> Option<u16> {
    if run.is_empty() {
        return None;
    }
    if run.starts_with(|c: char| c.is_ascii_digit()) {
        own.iter()
            .map(|(id, _)| *id)
            .find(|id| id.to_string().starts_with(run))
    } else {
        let run = run.to_uppercase();
        own.iter()
            .find(|(_, text)| {
                text.split_once(" - ")
                    .is_some_and(|(_, name)| name.starts_with(run.as_str()))
            })
            .map(|(id, _)| *id)
    }
}

/// The Nation row (`generalPanel`'s `nation`): a label-left row on the
/// same column as Name's (`label_col` wide, the field `field_width` to
/// the flush edge) — the open save's version's own nations `ID - NAME`
/// ordered by ID (`nations::nations`). An ID the version lacks keeps
/// its number, shown `ID - ?` first in the list until another is
/// picked. The wheel over the closed combo steps through the entries,
/// as Registered position's does. Returns a typed pick's row id: it
/// took the keyboard (`response.request_focus`), so the caller records
/// it as the held widget.
fn nation_row(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    held: u16,
    version: PesVersion,
    label_col: f32,
    field_width: f32,
) -> Option<egui::Id> {
    let own = nations(version);
    // The held ID's one lookup for the draw: `listed` is whether the
    // version lists it, `text` its `ID - NAME` — or `ID - ?`, also the
    // list's first row, where it does not.
    let held_at = own.binary_search_by_key(&held, |(id, _)| *id).ok();
    let listed = held_at.is_some();
    let text = held_at.map_or_else(
        || Cow::Owned(format!("{held} - ?")),
        |at| Cow::Borrowed(own[at].1.as_str()),
    );
    let mut focus = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let label = ui.label(NATION_LABEL);
        ui.add_space((label_col + LABEL_GAP - label.rect.width()).max(0.0));
        let mut picked = held;
        // The entries' `ID - NAME` texts are `nations()`'s, built once
        // on first read; only `ID - ?` is built per frame, for the closed
        // combo too. `open` is whether the list was open last frame: the
        // held row scrolls into view only on the frame it opens, so the
        // member can scroll away from it.
        let open = egui::Id::unique(("general nation list open", player_id));
        let was_open = ui.data(|data| data.get_temp(open)).unwrap_or(false);
        let combo = egui::ComboBox::from_id_salt("general nation")
            .selected_text(text.clone())
            .width(field_width)
            .show_ui(ui, |ui| {
                // A key typed while the list is open (a Text event;
                // egui-winit drops them while Ctrl or Cmd is held, so no
                // modifier check here) joins the run in progress: digits
                // an ID, letters a name — a space joins a name run under
                // way and is otherwise ignored, a key of the other kind
                // starts a new run, and any other text does nothing.
                // The run and its last key's time are the combo's temp
                // data, dropped as the list opens. The events the run
                // takes leave the input so nothing drawn after reads
                // them again.
                let typed = egui::Id::unique(("general nation list typed", player_id));
                let opened_with = egui::Id::unique(("general nation list opened with", player_id));
                if !was_open {
                    // The nation the list opened on is kept for the
                    // Escape below. A scroll asked for during a pass
                    // moves the rows only from the second pass after —
                    // the held row's open scroll would paint a frame
                    // late, the list at its top first; a discard pass
                    // folds the scroll's application into the open's own
                    // frame, and the first frame after the click shows
                    // it.
                    ui.data_mut(|data| data.insert_temp(opened_with, held));
                    ui.ctx().request_discard("the nation list's opening scroll");
                }
                let now = ui.input(|input| input.time);
                let (mut run, mut at) = ui
                    .data(|data| data.get_temp::<(String, f64)>(typed))
                    .filter(|(_, at)| was_open && now - at <= RUN_PAUSE)
                    .unwrap_or_default();
                let mut used = false;
                ui.input_mut(|input| {
                    input.events.retain(|event| {
                        let name_run = run.starts_with(|c: char| !c.is_ascii_digit());
                        let digit_run = run.starts_with(|c: char| c.is_ascii_digit());
                        let take = match event {
                            egui::Event::Text(text) if text.chars().all(|c| c.is_ascii_digit()) => {
                                if name_run {
                                    run.clear();
                                }
                                run.push_str(text);
                                true
                            }
                            egui::Event::Text(text) if text.chars().all(|c| c.is_alphabetic()) => {
                                if digit_run {
                                    run.clear();
                                }
                                run.push_str(text);
                                true
                            }
                            egui::Event::Text(text)
                                if name_run && text.chars().all(|c| c == ' ') =>
                            {
                                run.push_str(text);
                                true
                            }
                            _ => false,
                        };
                        used |= take;
                        !take
                    });
                });
                if used {
                    at = now;
                }
                // The pick applies before any row draws, so one row
                // reads selected on the frame it lands.
                let typed_pick = if used { run_pick(own, &run) } else { None };
                ui.data_mut(|data| data.insert_temp(typed, (run, at)));
                if let Some(next) = typed_pick {
                    picked = next;
                }
                if !listed {
                    let response = combo_item(ui, &mut picked, held, text.clone());
                    if !was_open {
                        response.scroll_to_me_animation(
                            Some(egui::Align::Center),
                            egui::style::ScrollAnimation::none(),
                        );
                    }
                }
                for (id, row_text) in own {
                    let response = combo_item(ui, &mut picked, *id, row_text);
                    if !was_open && *id == held {
                        response.scroll_to_me_animation(
                            Some(egui::Align::Center),
                            egui::style::ScrollAnimation::none(),
                        );
                    }
                    if typed_pick == Some(*id) {
                        // Enter after a typed run activates the picked
                        // row, not wherever arrows left the keyboard
                        // focus.
                        response.request_focus();
                        focus = Some(response.id);
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                }
                // Escape puts back the nation the list opened on — the
                // popup closes on it by itself — so the picks' undo
                // entry ends where it started, with nothing left to
                // take back.
                if was_open && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                    picked = ui.data(|data| data.get_temp(opened_with)).unwrap_or(held);
                }
            });
        ui.data_mut(|data| data.insert_temp(open, combo.inner.is_some()));
        let ids = (!listed)
            .then_some(held)
            .into_iter()
            .chain(own.iter().map(|(id, _)| *id));
        if let Some(next) = step_choice(ids, &picked, wheel_steps(ui, &combo.response)) {
            picked = next;
        }
        if picked != held {
            edit_player(session, player_id, combo.response.id.value(), |player| {
                player.basic.nationality = picked;
            });
        }
    });
    focus
}

/// `small`'s box for a pair half: the shifted `DragValue` (a stored value
/// past the shown range drawn as stored). `SmallStat::get` stays an
/// `Option` for the Appearance section's gated stats; the Main stats
/// panel's four are stored on every version, so the gated `-` stand-in
/// has no draw here.
fn small_box(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    stats: &PlayerStats,
    small: &SmallStat,
) {
    let stored = (small.get)(stats).expect("SMALL_STATS' four fields are stored on every version");
    let mut shown = stored + small.shift;
    let range = small.min..=small_stat_max(session.version(), small);
    let mut response = number_arrows(ui, &mut shown, range.clone(), |ui, value| {
        ui.add_sized(
            [BOX_WIDTH, ui.spacing().interact_size.y],
            studio_core::widgets::number_box(
                egui::DragValue::new(value)
                    .range(range.clone())
                    .clamp_existing_to_range(false),
            ),
        )
    });
    if step_number(&mut shown, number_wheel_steps(ui, &response), range) {
        response.mark_changed();
    }
    if response.changed() {
        edit_player(session, player_id, response.id.value(), move |player| {
            (small.set)(&mut player.stats, shown - small.shift);
        });
    }
}

/// Main stats' four rows — the foot and hand toggles, the position and
/// style combos — label-left on the same label column as Name's and
/// Shirt name's (`generalPanel`'s `.combos` `109px` grid: its widest
/// label is the column's "Registered position"), each `field_width`
/// wide so it ends at the flush edge with the second half's boxes.
/// `marks` is Live AATF's held check's: the marked rows' marks stand
/// before their labels.
fn combos(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    positions: &PlayerPositions,
    version: PesVersion,
    field_width: f32,
    marks: Option<Marks<'_>>,
) {
    egui::Grid::new("general combos")
        .num_columns(2)
        .spacing(egui::vec2(LABEL_GAP, FIELD_SPACING))
        .show(ui, |ui| {
            ui.label(STRONGER_FOOT_LABEL);
            if let Some((side, key)) = side_row(ui, field_width, positions.stronger_foot) {
                edit_player(session, player_id, key, |player| {
                    player.positions.stronger_foot = side;
                });
            }
            ui.end_row();
            if positions.stronger_hand.is_some() {
                ui.label(STRONGER_HAND_LABEL);
            } else {
                gated_label(
                    ui,
                    STRONGER_HAND_LABEL,
                    gating_tooltip(PlayerField::StrongerHand),
                );
            }
            match positions.stronger_hand {
                Some(stored) => {
                    if let Some((side, key)) = side_row(ui, field_width, stored) {
                        edit_player(session, player_id, key, |player| {
                            player.positions.stronger_hand = Some(side);
                        });
                    }
                }
                None => {
                    // The version has no stronger hand: the toggle stays,
                    // at Right, disabled with the `PES NN+` tooltip
                    // (SE-GUI-06).
                    add_enabled_ui(ui, false, |ui| {
                        side_row(ui, field_width, 0);
                    })
                    .response
                    .on_disabled_hover_text(gating_tooltip(PlayerField::StrongerHand));
                }
            }
            ui.end_row();
            let registered_label = ui.label(REGISTERED_POSITION_LABEL);
            if let Some(marks) = marks {
                marks.paint(ui, registered_label.rect, MarkedField::RegisteredPosition);
            }
            let mut registered = positions.registered;
            let combo = egui::ComboBox::from_id_salt("abilities registered position")
                .selected_text(position_label(registered))
                .width(field_width)
                .show_ui(ui, |ui| {
                    for index in 0..POSITIONS.len() {
                        combo_item(
                            ui,
                            &mut registered,
                            index as u8,
                            position_label(index as u8),
                        );
                    }
                });
            let values = (0..=u8::MAX).take(POSITIONS.len());
            let steps = wheel_steps(ui, &combo.response);
            if let Some(next) = step_choice(values, &registered, steps) {
                registered = next;
            }
            if registered != positions.registered {
                edit_player(session, player_id, combo.response.id.value(), |player| {
                    player.positions.registered = registered;
                });
            }
            ui.end_row();
            let style_label = ui.label(PLAYING_STYLE_LABEL);
            if let Some(marks) = marks {
                marks.paint(ui, style_label.rect, MarkedField::PlayingStyle);
            }
            ui.scope(|ui| {
                ui.spacing_mut().combo_width = field_width;
                let mut style = positions.playing_style;
                if cards::playing_style_picker(ui, "abilities playing style", version, &mut style) {
                    edit_player(
                        session,
                        player_id,
                        egui::Id::unique(("abilities playing style", player_id)).value(),
                        |player| {
                            player.positions.playing_style = style;
                        },
                    );
                }
            });
            ui.end_row();
        });
}

/// One row of a `.rate` toggle's segments: one `Button::selectable` per
/// `(value, label, width)`, a point apart, the held one filled the
/// positions grid's C grey with the strong text colour, the others'
/// words in the normal text grey (the draft's `.sel.on` and
/// `.sel:not(.on):hover`). In a disabled scope both colours give way to
/// the scope's dimmed text, as a disabled control's words do. An unheld
/// segment is a resting button, whose
/// own grey is lighter than the normal text grey the draft wants here:
/// the resting colour is set to the normal text grey inside the row,
/// where only these segments draw, and a hover still takes the hovered
/// state's brighter grey. `small` makes the segments egui's small
/// buttons (their words' height, no vertical padding: the draft's
/// `.sel` segments); off, each is at least a control's height. Returns
/// the picked segment's value and its button's id value (the undo merge
/// key) on a click, `None` on the held segment's.
pub(super) fn segment_row<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    segments: &[(T, &'static str, f32)],
    held: T,
    small: bool,
) -> Option<(T, u64)> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        let strong = ui.visuals().strong_text_color();
        if ui.is_enabled() {
            ui.visuals_mut().widgets.inactive.fg_stroke.color = ui.visuals().text_color();
        } else {
            // A selected button's word is the selection's stroke: a
            // disabled held side's word dims like every disabled
            // control's.
            ui.visuals_mut().selection.stroke.color = ui.visuals().widgets.inactive.fg_stroke.color;
        }
        // The resting edges a toggle's sides paint: the held side's edge
        // is its fill's own (`edge_color`), an unheld side's the panel's
        // (`unheld_edge`) — a disabled segment draws none.
        let held_edge = studio_core::theme::edge_color(
            ui.visuals(),
            studio_core::theme::palette(ui.visuals()).rating_c_fill,
        );
        let unheld = studio_core::theme::unheld_edge(ui.visuals());
        for (value, label, width) in segments {
            let on = *value == held;
            // The held side's strong word is the live look: disabled it
            // takes the dimmed selection stroke above.
            let text: egui::WidgetText = if on && ui.is_enabled() {
                egui::RichText::new(*label).color(strong).into()
            } else {
                (*label).into()
            };
            let mut button = egui::Button::selectable(on, text);
            if on {
                button = button.fill(studio_core::theme::palette(ui.visuals()).rating_c_fill);
            }
            let button = if small { button.small() } else { button };
            let response = ui.add_sized([*width, ui.spacing().interact_size.y], button);
            resting_edge(ui, &response, if on { held_edge } else { unheld });
            if response.clicked() && !on {
                picked = Some((*value, response.id.value()));
            }
        }
    });
    picked
}

/// The `Left | Right` two-choice toggle of the `Stronger foot`/`Stronger
/// hand` rows (the draft's `.rate.lr`: the two choices equal halves of
/// `field_width`, `segment_row`'s point apart, the held one filled the
/// positions grid's C grey — `Button::selectable`, as the grid's
/// ratings draw).
/// The held side's word is the strong text colour, the other side's the
/// normal text grey, so the chosen side reads white; the other side's
/// hover still brightens it as a button's does. `FOOT` stores right 0
/// and left 1; the toggle draws Left first. Returns the picked side's
/// stored index and its button's id value (the undo merge key) on a
/// click.
fn side_row(ui: &mut egui::Ui, field_width: f32, held: u8) -> Option<(u8, u64)> {
    let side = (field_width - 1.0) / 2.0;
    segment_row(
        ui,
        &[(1u8, "Left", side), (0u8, "Right", side)],
        held,
        false,
    )
}

/// How near `limit` a count of `n` is, as its colour: the grey under it,
/// the warning colour on it, the error colour past it.
fn count_color(visuals: &egui::Visuals, n: usize, limit: usize) -> egui::Color32 {
    match n.cmp(&limit) {
        std::cmp::Ordering::Less => studio_core::theme::palette(visuals).count_grey,
        std::cmp::Ordering::Equal => visuals.warn_fg_color,
        std::cmp::Ordering::Greater => visuals.error_fg_color,
    }
}

/// The `label (n/max)` line painted at `rect` under `id` (allocated before
/// the field was drawn, so the count follows the field's text as typed
/// this frame): `label` in the normal text colour, `count`'s `(n/max)` in
/// `count_color`'s against `limit` (the smaller of the shown maximum and
/// the portable one). Returns the label's response (the AccessKit node for
/// the hover it shares with the field).
fn counted_label(
    ui: &mut egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    label: &str,
    count: (usize, usize),
    limit: usize,
) -> egui::Response {
    let (n, max) = count;
    let text_color = ui.visuals().text_color();
    let body = egui::TextStyle::Body.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &format!("{label} "),
        0.0,
        egui::TextFormat::simple(body.clone(), text_color),
    );
    job.append(
        &format!("({n}/{max})"),
        0.0,
        egui::TextFormat::simple(body, count_color(ui.visuals(), n, limit)),
    );
    let galley = ui.painter().layout_job(job);
    // `interact`, not `allocate_rect`: the space was taken before the
    // field drew, so this must not move the cursor again.
    let response = ui.interact(rect, id, egui::Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::accesskit::Role::Label,
            true,
            format!("{label} ({n}/{max})"),
        )
    });
    ui.painter().galley(rect.min, galley, text_color);
    response
}

/// The tint of a field whose text a version would crop: its usual
/// background drawn a quarter toward the warning colour.
fn warning_tint(visuals: &egui::Visuals) -> egui::Color32 {
    visuals
        .extreme_bg_color
        .lerp_to_gamma(visuals.warn_fg_color, 0.25)
}

/// The biggest value the save's `schema` stores in `field`'s player-record
/// bit run (`2^width - 1`, the codec's `ValueTooWide` bound): the range the
/// strip's number controls take, so a typed value is never one the codec
/// refuses. The strip's fields are stored on every version.
fn max(schema: &VersionSchema, field: PlayerField) -> u64 {
    schema
        .player
        .max_value(field)
        .expect("the strip's fields are stored on every version")
}

/// `field`'s range in the shown values the model holds: the stored bound
/// with the version's stored offset at both ends, since the codec takes
/// the offset off on write. PES 2014's height runs 132 to 387 and its
/// weight 30 to 285; every other version's stays the stored bound itself.
fn shown_range(schema: &VersionSchema, field: PlayerField) -> std::ops::RangeInclusive<u64> {
    let offset = u64::from(stored_offset(schema.version, field));
    offset..=max(schema, field) + offset
}

/// The length of the player record's `which` text field, from the schema
/// (the Name field's bytes, and the Shirt field's char count).
fn text_len(schema: &VersionSchema, which: PlayerText) -> usize {
    schema
        .player
        .texts
        .iter()
        .find(|spec| spec.text == which)
        .map(|spec| usize::try_from(spec.len).expect("a field length fits usize"))
        .expect("every version stores the text")
}

/// The same bound for the roster's shirt-number run.
fn roster_number_max(schema: &VersionSchema) -> u16 {
    let max = schema
        .roster
        .max_value(RosterField::Number(0))
        .expect("every version stores the shirt number");
    u16::try_from(max).expect("the shirt number's bound fits u16")
}
