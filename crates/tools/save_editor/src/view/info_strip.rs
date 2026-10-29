//! The player view's title and the first column's panels (the draft's
//! `playerHeader`, `generalPanel` and `otherStatsPanel`): `<id> <name>` at
//! the heading size, then General info — Name and Shirt, each a
//! label-left row carrying its `(n/max)` count beside its field, the
//! label-left pairs on two halves (each box 54 wide: on the first half's
//! right edge, after the label column on the second), then the combos
//! whose labels share one column — and Other stats under it on the same
//! halves so every box lines up. Each control edits a copy of the entry
//! and, when it changes, pushes it under its own `egui::Id` (the undo
//! stack's merge key).

use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::names::{crop_player_text, name_fits, shirt_name_edit_fits};
use pes_savefile::model::{PlayerEntry, PlayerPositions, PlayerStats};
use pes_savefile::schema::fields::{PlayerField, PlayerText, RosterField};
use pes_savefile::schema::{VersionSchema, portable_text_max, schema_for};
use pes_version::PesVersion;
use studio_core::widgets::{FORM_ROW_SPACING, radio, step_choice, step_number, wheel_steps};
use team_widgets::cards;
use team_widgets::names::append_name;

use crate::session::Session;
use crate::view::abilities::{
    PLAYING_STYLE_LABEL, REGISTERED_POSITION_LABEL, SMALL_STATS, STRONGER_FOOT_LABEL,
    STRONGER_HAND_LABEL, SmallStat, gating_tooltip, position_label,
};
use crate::view::live_marks::{MarkedField, Marks};
use crate::view::{SaveView, edit_player, edit_team, selected_player};

/// The General info panel's Name field.
pub(super) const NAME_LABEL: &str = "Name";
/// The General info panel's Shirt name field.
pub(super) const SHIRT_LABEL: &str = "Shirt";
/// The General info panel's Height field.
pub(super) const HEIGHT_LABEL: &str = "Height";
/// The General info panel's Weight field.
pub(super) const WEIGHT_LABEL: &str = "Weight";
/// The General info panel's Age field.
pub(super) const AGE_LABEL: &str = "Age";
/// The General info panel's Nation field.
pub(super) const NATION_LABEL: &str = "Nation";
/// The General info panel's Shirt number field.
pub(super) const NUMBER_LABEL: &str = "Shirt number";

/// General info's fields' width (the draft's 344px: room for the longest
/// pair label, "Injury resistance", beside its box on each half and for
/// the label-left rows; no wider, so the player view's three columns stay
/// on one row beside the 255-point list).
pub(super) const GENERAL_WIDTH: f32 = 344.0;
/// A pair's box width (the draft's `.pair` fields).
const BOX_WIDTH: f32 = 54.0;
/// The gap between the second half's label column and its box (the
/// draft's `.pair:nth-child(even)` `column-gap`, 9 points).
const SECOND_LABEL_GAP: f32 = 9.0;
/// The gap between a label-left row's label column and its field (the
/// draft's `.combos` `column-gap`, 12 points; Ability stats' rows share
/// it).
pub(super) const FIELD_ROW_GAP: f32 = 12.0;
/// The gap between a pair row's two halves (the draft's `.pairs`
/// `column-gap`, 28 points).
const PAIR_HALF_GAP: f32 = 28.0;
/// The gap between General info's blocks (Name and Shirt, the pairs, the
/// combos): the draft's `.gen` gap, 2 points more than the rows' own, so
/// each block reads as one.
const BLOCK_GAP: f32 = 6.0;

/// A player name as the strip draws it: its `\x11` spelled as the four
/// characters `\\x11` (the draft's `escName`); the field turns it back on
/// edit.
pub(super) fn shown_name(name: &str) -> String {
    name.replace('\x11', "\\x11")
}

/// The header for the selected player; nothing when the selection names no
/// rostered player (a stale slot, an empty save's list).
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
    // General info's title holds its ink higher in its row than the team
    // view's labels, centred in their 18-point row, so this view's first
    // row needs more room under the title for even ink gaps.
    view_title(ui, player_id, &name, 4.0);
}

/// A view's title, the player view's and the team view's alike (the
/// draft's `viewTitle`): `<id> <name>`, not the tool's name, which the
/// title bar carries. The id in the weak colour as the list draws ids, the
/// name's colour codes drawn as the list draws them (`append_name`), its
/// plain runs in the strong colour, all in the body text's size (the app
/// installs no bold face). `space_below` comes after it: the view's first
/// row's ink then stands as far below the title's ink as the title's
/// stands below the toolbar's buttons. Even boxes would not do it, since a
/// line box holds more room under the baseline than over the capitals.
pub(super) fn view_title(ui: &mut egui::Ui, id: u32, name: &str, space_below: f32) {
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
    ui.add_space(space_below);
}

/// The General info panel's content (`generalPanel`): the Name and Shirt
/// counted fields, the pair rows, then the combos. `marks` is Live
/// AATF's held check's, while it is the selected player's team's: each
/// marked field's mark then stands before its label (`Marks::paint`).
pub(super) fn general(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    slot: u8,
    player_id: u32,
    marks: Option<Marks<'_>>,
) {
    let (
        mut name,
        stored_name,
        mut shirt,
        stored_shirt,
        nation,
        mut shirt_number,
        basics,
        stored_captain,
        positions,
        schema,
    ) = {
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
            (player.basic.height, player.basic.weight, player.basic.age),
            team.tactics.set_pieces.captain,
            player.positions.clone(),
            schema_for(file.version()),
        )
    };
    let captain = stored_captain == slot;
    let (height, weight, age) = basics;

    // The rows' layout, measured once (the draft's `.combos` `width`):
    // the label column, its 12-point gap, the fields to the flush edge —
    // the first half (label column, gap, box), the 28, the second half's
    // label column, its 9 and a box. Past `GENERAL_WIDTH` the panel
    // overflows rather than shrinking a gap.
    let label_col = label_width(ui, REGISTERED_POSITION_LABEL);
    let field_x = ui.max_rect().min.x + label_col + FIELD_ROW_GAP;
    let flush_x = ui.max_rect().min.x + general_content_width(ui);
    let field_width = flush_x - field_x;

    // Each a label-left row: the `label (n/max)` label's rect taken at
    // the row's left, the label itself painted after the field — the
    // count follows the field's own text this frame (a TextEdit writes
    // its buffer during `ui.add`), not the stored name. The field and
    // its count carry one hover.
    let overlong = stored_name.len() > portable_text_max(PlayerText::Name);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = FIELD_ROW_GAP;
        let (name_label_id, name_label) = ui.allocate_space(egui::vec2(
            label_col,
            ui.text_style_height(&egui::TextStyle::Body),
        ));
        let mut name_edit = egui::TextEdit::singleline(&mut name);
        if overlong {
            name_edit = name_edit.background_color(warning_tint(ui.visuals()));
        }
        let response = ui.add_sized([field_width, ui.spacing().interact_size.y], name_edit);
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
                "Up to {} bytes. A plain letter takes one byte, an accented letter takes two or \
                 more, and a colour code takes ten. A colour code is typed as \\x11c and eight \
                 hex digits.",
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
        ui.spacing_mut().item_spacing.x = FIELD_ROW_GAP;
        let (shirt_label_id, shirt_label) = ui.allocate_space(egui::vec2(
            label_col,
            ui.text_style_height(&egui::TextStyle::Body),
        ));
        let mut shirt_edit = egui::TextEdit::singleline(&mut shirt);
        if shirt_overlong {
            shirt_edit = shirt_edit.background_color(warning_tint(ui.visuals()));
        }
        let response = ui.add_sized([field_width, ui.spacing().interact_size.y], shirt_edit);
        let tip = if shirt_overlong {
            "PES 2019 and older show 18 letters of a shirt name (PES 2016 shows 16)".to_owned()
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

    // The label-left pairs on two halves: each label at its half's left,
    // its 54-wide box on the first half's right edge, after the label
    // column on the second.
    ui.add_space(BLOCK_GAP - ui.spacing().item_spacing.y);
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
                    max(schema, PlayerField::Height),
                    |player, shown| player.basic.height = shown,
                );
            },
        );
        pair_half(ui, Half::Second, NUMBER_LABEL, None, |ui| {
            let range = 0..=roster_number_max(schema);
            let mut response = ui.add_sized(
                [BOX_WIDTH, ui.spacing().interact_size.y],
                egui::DragValue::new(&mut shirt_number).range(range.clone()),
            );
            if step_number(&mut shirt_number, wheel_steps(ui, &response), range) {
                response.mark_changed();
            }
            if response.changed() {
                edit_team(session, team_id, response.id.value(), |team| {
                    team.roster[usize::from(slot)].number = shirt_number;
                });
            }
        });
    });
    pair_row(ui, |ui| {
        pair_half(
            ui,
            Half::First,
            WEIGHT_LABEL,
            marks.map(|marks| (marks, MarkedField::Weight)),
            |ui| {
                number_box(
                    ui,
                    session,
                    player_id,
                    weight,
                    max(schema, PlayerField::Weight),
                    |player, shown| player.basic.weight = shown,
                );
            },
        );
        pair_half(ui, Half::Second, NATION_LABEL, None, |ui| {
            number_box(
                ui,
                session,
                player_id,
                nation,
                max(schema, PlayerField::Nationality),
                |player, shown| player.basic.nationality = shown,
            );
        });
    });
    pair_row(ui, |ui| {
        pair_half(
            ui,
            Half::First,
            AGE_LABEL,
            marks.map(|marks| (marks, MarkedField::Age)),
            |ui| {
                number_box(
                    ui,
                    session,
                    player_id,
                    age,
                    max(schema, PlayerField::Age),
                    |player, shown| player.basic.age = shown,
                );
            },
        );
        pair_half(ui, Half::Second, "Captain", None, |ui| {
            // The team's `captain` is this roster slot, 0-based as stored: a
            // radio the second click cannot untick (the captain belongs to
            // the slot) — centred in its 54-wide box, under the Nation box
            // (the draft's `.pair > .box` `justify-content: center`).
            ui.allocate_ui(egui::vec2(BOX_WIDTH, ui.spacing().interact_size.y), |ui| {
                ui.centered_and_justified(|ui| {
                    let response = radio(ui, captain, "");
                    if response.clicked() {
                        edit_team(session, team_id, response.id.value(), |team| {
                            team.tactics.set_pieces.captain = slot;
                        });
                    }
                });
            });
        });
    });

    // The combos, on the same label-left rows as Name's and Shirt's.
    ui.add_space(BLOCK_GAP - ui.spacing().item_spacing.y);
    combos(
        ui,
        session,
        player_id,
        &positions,
        schema.version,
        field_width,
        marks,
    );
}

/// One row of the pairs' two halves, `PAIR_HALF_GAP` points apart.
fn pair_row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = PAIR_HALF_GAP;
        add(ui);
    });
}

/// Which of a pair row's two halves a pair sits on: the first takes the
/// half's own width and puts its box on its right edge, the second — the
/// row's last item — takes only what its content draws, just after its
/// label column.
#[derive(Clone, Copy)]
enum Half {
    /// The row's left half (the draft's `.pair` flex row).
    First,
    /// The row's right half (the draft's `.pair:nth-child(even)` grid).
    Second,
}

/// One `label`-left pair of a `pair_row`: the label at the half's left;
/// on `Half::First` `add_box`'s 54-wide box on the half's right edge (the
/// half is the label-left rows' label column, their 12 and a box wide,
/// so its boxes start at the x those rows' fields start at), on
/// `Half::Second` just after the `label_width(NUMBER_LABEL)` label
/// column and its gap, so both panels' second-half boxes share one x.
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
            let width = label_width(ui, REGISTERED_POSITION_LABEL) + FIELD_ROW_GAP + BOX_WIDTH;
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
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let response = ui.label(label);
                let column = label_width(ui, NUMBER_LABEL);
                // The draft's `.pair:nth-child(even)` grid: the box after a
                // label column `label_width(NUMBER_LABEL)` wide and its gap, so
                // both panels' second-half boxes share one x. `add_space`
                // advances the cursor outright, no item spacing.
                ui.add_space((column + SECOND_LABEL_GAP - response.rect.width()).max(0.0));
                if let Some((marks, field)) = mark {
                    marks.paint(ui, response.rect, field);
                }
                add_box(ui);
            });
        }
    }
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

/// The General info column's content width: the flush edge the label-left
/// rows' fields and the pairs' second-half boxes draw to — the rows'
/// label column, the 12, a box, the 28, the second half's label column,
/// the 9 and a box (the draft's `.combos` `width`). Wider than
/// `GENERAL_WIDTH` if egui's text runs wide; the panel overflows its
/// nominal 344 then rather than shrinking a gap.
pub(super) fn general_content_width(ui: &egui::Ui) -> f32 {
    // The rows' label column is the longest of their six labels,
    // "Registered position"; the second half's its longest, "Shirt number".
    label_width(ui, REGISTERED_POSITION_LABEL)
        + label_width(ui, NUMBER_LABEL)
        + FIELD_ROW_GAP
        + 2.0 * BOX_WIDTH
        + PAIR_HALF_GAP
        + SECOND_LABEL_GAP
}

/// One pair's 54-wide `DragValue` over `shown` capped at `max`; a change
/// pushes the player's entry with `set` applied.
fn number_box<N: egui::emath::Numeric + TryFrom<u64>>(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    shown: N,
    max: u64,
    set: impl FnOnce(&mut PlayerEntry, N),
) {
    let mut shown = shown;
    let range = N::from_f64(0.0)..=N::try_from(max).unwrap_or(N::MAX);
    let mut response = ui.add_sized(
        [BOX_WIDTH, ui.spacing().interact_size.y],
        egui::DragValue::new(&mut shown).range(range.clone()),
    );
    if step_number(&mut shown, wheel_steps(ui, &response), range) {
        response.mark_changed();
    }
    if response.changed() {
        edit_player(session, player_id, response.id.value(), |player| {
            set(player, shown);
        });
    }
}

/// The Other stats panel's content (`otherStatsPanel`): the four shifted
/// stored fields on General info's halves, so every box of both panels
/// lines up — Injury resistance with Form, then "Weak foot: Usage" and
/// "Accuracy" (the two sharing one label). `marks` is Live AATF's held
/// check's, while it is the selected player's team's.
pub(super) fn other_stats(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    stats: &PlayerStats,
    marks: Option<Marks<'_>>,
) {
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
}

/// `small`'s box for a pair half: the shifted `DragValue` for a field the
/// version stores (a stored value past the shown range drawn as stored),
/// the disabled `-` for one it lacks.
fn small_box(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    stats: &PlayerStats,
    small: &SmallStat,
) {
    match (small.get)(stats) {
        Some(stored) => {
            let mut shown = stored + small.shift;
            let mut response = ui.add_sized(
                [BOX_WIDTH, ui.spacing().interact_size.y],
                egui::DragValue::new(&mut shown)
                    .range(small.min..=small.max)
                    .clamp_existing_to_range(false),
            );
            if step_number(
                &mut shown,
                wheel_steps(ui, &response),
                small.min..=small.max,
            ) {
                response.mark_changed();
            }
            if response.changed() {
                edit_player(session, player_id, response.id.value(), move |player| {
                    (small.set)(&mut player.stats, shown - small.shift);
                });
            }
        }
        None => {
            ui.add_enabled(
                false,
                egui::Button::new("-")
                    .min_size(egui::vec2(BOX_WIDTH, ui.spacing().interact_size.y)),
            )
            .on_disabled_hover_text(gating_tooltip(small.field));
        }
    }
}

/// The four combos, label-left rows on the same label column as Name's
/// and Shirt's (`generalPanel`'s `.combos` `107px` grid: its widest label
/// is the column's "Registered position"), each `field_width` wide so it
/// ends at the flush edge with the second half's boxes. `marks` is Live
/// AATF's held check's: the marked combos' marks stand before their
/// labels.
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
        .spacing(egui::vec2(FIELD_ROW_GAP, FORM_ROW_SPACING))
        .show(ui, |ui| {
            ui.label(STRONGER_FOOT_LABEL);
            if let Some((side, key)) = side_row(ui, field_width, positions.stronger_foot) {
                edit_player(session, player_id, key, |player| {
                    player.positions.stronger_foot = side;
                });
            }
            ui.end_row();
            ui.label(STRONGER_HAND_LABEL);
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
                    ui.add_enabled_ui(false, |ui| {
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
                        ui.selectable_value(
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

/// The `Left | Right` two-choice toggle of the `Stronger foot`/`Stronger
/// hand` rows (the draft's `.rate.lr`: the two choices equal halves of
/// `field_width`, a point apart, the held one filled the positions
/// grid's C grey with the normal text colour — `Button::selectable`, as
/// the grid's ratings draw). `FOOT` stores right 0 and left 1; the
/// toggle draws Left first. Returns the picked side's stored index and
/// its button's id value (the undo merge key) on a click.
fn side_row(ui: &mut egui::Ui, field_width: f32, held: u8) -> Option<(u8, u64)> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        let side = (field_width - ui.spacing().item_spacing.x) / 2.0;
        for (label, stored) in [("Left", 1u8), ("Right", 0u8)] {
            let held = held == stored;
            let button = if held {
                egui::Button::selectable(
                    true,
                    egui::RichText::new(label).color(ui.visuals().text_color()),
                )
                .fill(studio_core::theme::palette(ui.visuals()).rating_c_fill)
            } else {
                egui::Button::selectable(false, label)
            };
            let response = ui.add_sized([side, ui.spacing().interact_size.y], button);
            if response.clicked() && !held {
                picked = Some((stored, response.id.value()));
            }
        }
    });
    picked
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
