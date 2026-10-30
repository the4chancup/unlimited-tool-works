//! The team view (the draft's `teamView`): the selected team's id and name
//! with its colours as the title, one row holding the name, short name and colours
//! 1-2 fields (the former team bar's pairs), the manager and stadium IDs
//! and the `Kit slots…` button opening the kit slot assignments in a
//! dialog, then the Tactics editor. Every control edits a copy of the
//! team's entry and,
//! when it changes, pushes it with `Session::edit_team` under the control's
//! `egui::Id` (the undo stack's merge key, as `info_strip.rs` does). A
//! field the version does not store is drawn disabled in its place, its
//! tooltip naming the first version that has it (the kit slots PES 17
//! alone has); a value the save holds outside a control's range shows as
//! stored and changes only when edited (the Abilities rule: drawing never
//! edits).

use std::ops::RangeInclusive;

use pes_savefile::model::names::team_text_fits;
use pes_savefile::model::{TeamColor, TeamEntry};
use pes_savefile::schema::fields::{TeamField, TeamText};
use pes_savefile::schema::{VersionSchema, first_version_with_team, schema_for};
use pes_version::PesVersion;
use studio_core::widgets::{
    FORM_ROW_SPACING, add_enabled, add_enabled_ui, modal, modal_buttons, step_number, text_box,
    wheel_steps,
};

use crate::session::Session;
use crate::view::{SaveView, edit_team, info_strip, selected_team, tactics};

/// One unit of the header's row: a label with its field, or the button.
#[derive(Debug, Clone, Copy)]
pub(super) enum HeaderUnit {
    /// The `Name` pair.
    Name,
    /// The `Short` pair.
    Short,
    /// `Col. 1` (0) or `Col. 2` (1), its picker and hex.
    Colour(usize),
    /// The `Manager ID` pair.
    Manager,
    /// The `Stadium ID` pair.
    Stadium,
    /// The `Kit slots…` button.
    KitSlots,
}

/// The header's units in their order, each with its claimed width on the
/// row, measured once in a headless frame at 1600 x 1400 (each unit's
/// drawn `min_rect` width; the widths are fixed, a longer team name
/// changes nothing), each unit one `allocate_ui` at its claim.
pub(super) const TEAM_ROW: [(HeaderUnit, f32); 7] = [
    (HeaderUnit::Name, 182.0),
    (HeaderUnit::Short, 79.0),
    (HeaderUnit::Colour(0), 111.0),
    (HeaderUnit::Colour(1), 111.0),
    (HeaderUnit::Manager, 114.5),
    (HeaderUnit::Stadium, 111.0),
    (HeaderUnit::KitSlots, 64.0),
];
/// A space at the body size: the gap the draft's text leaves between a
/// colour's label, picker and hex, and between an id's label and its box.
const SPACE_GAP: f32 = 3.0;
/// The padding on each side of the draft's id cells (the Manager and
/// Stadium IDs are cells of one grid there).
const ID_CELL_PADDING: f32 = 4.0;
/// The width of an id's box: the draft's `.info-grid` number box (egui's
/// own is 40). The number control and its disabled stand-in both take
/// their width from `interact_size`, so the id units set it.
const ID_BOX_WIDTH: f32 = 46.0;
/// The room under the title, past the item spacing: the fields' row's
/// boxes stand 6 under the title's line box, so the row's ink stands as
/// far under the title's ink as the title's under the toolbar's buttons
/// (within half a point), and the row lands on a whole point, where the
/// line under it paints one pixel row. The draft's boxes stand 7 under,
/// which leaves its ink gap below the title wider than the one above.
const TITLE_GAP: f32 = 3.0;
/// From the row's controls down to the row the line under them paints
/// (the draft's `.sep`, a 1-point border covering one pixel row): the
/// line's 8-point margin past the row's box, which stands 3 under its
/// controls.
const LINE_ABOVE: f32 = 11.0;
/// From the line's painted row's bottom down to the Tactics' top: the
/// `.sep`'s 8-point margin. The Tactics bar is taller than its controls,
/// so its first control stands 11.5 under the line's row.
const LINE_BELOW: f32 = 8.0;

/// The room the id cells' padding adds before `unit`, past the row's gap
/// `row_gap`: 12 from the colours to Manager ID, 8 between the two cells
/// (their padding alone, no row gap) and 12 from Stadium ID to the button.
fn id_cell_padding_before(unit: HeaderUnit, row_gap: f32) -> f32 {
    match unit {
        HeaderUnit::Name | HeaderUnit::Short | HeaderUnit::Colour(_) => 0.0,
        HeaderUnit::Manager | HeaderUnit::KitSlots => ID_CELL_PADDING,
        HeaderUnit::Stadium => 2.0 * ID_CELL_PADDING - row_gap,
    }
}

/// The gap between the parts of `unit`: the row's gap `row_gap` between
/// the Name and Short labels and their boxes (the draft lays those out as
/// units of the row), a space inside a colour or an id.
fn parts_gap(unit: HeaderUnit, row_gap: f32) -> f32 {
    match unit {
        HeaderUnit::Name | HeaderUnit::Short | HeaderUnit::KitSlots => row_gap,
        HeaderUnit::Colour(_) | HeaderUnit::Manager | HeaderUnit::Stadium => SPACE_GAP,
    }
}

/// The view's name field (the title already says it is the team's).
pub(super) const TEAM_LABEL: &str = "Name";
/// The view's Short field.
pub(super) const SHORT_LABEL: &str = "Short";
/// The two colours' labels (the draft's `Col.` shortening).
pub(super) const COLOUR_LABELS: [&str; 2] = ["Col. 1", "Col. 2"];
/// The view's Manager ID field.
pub(super) const MANAGER_LABEL: &str = "Manager ID";
/// The view's Stadium ID field.
pub(super) const STADIUM_LABEL: &str = "Stadium ID";
/// The `Kit slots…` button.
pub(super) const KIT_SLOTS_LABEL: &str = "Kit slots\u{2026}";
/// A kit slot's Number field.
pub(super) const KIT_NUMBER_LABEL: &str = "Number";
/// A kit slot's bound team field.
pub(super) const BOUND_TEAM_LABEL: &str = "Bound team ID";

/// The team view for the selected team: `<id> <name>` as the title
/// (`info_strip::view_title`), one wrapped row holding the name, short name and
/// colour pairs, the manager and stadium IDs and the `Kit slots…` button
/// (whole units wrap to a second row where the view is narrower, a pair
/// never splits), then the Tactics editor. Nothing when the selection
/// names no team of the save.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some((team_id, version, team)) = selected_team(view) else {
        return;
    };
    let generation = view.generation;
    let schema = schema_for(version);
    // Every widget id under the view is the team's within this open: a
    // text field's undo history, a DragValue's in-progress edit, a combo's
    // popup and the pickers' state never cross teams — nor saves (the
    // `generation` half), nor the player view's, the kind in the salt (a
    // save outside 4cc conventions can give a rostered player a team's
    // id). The session is re-taken per unit so the Tactics editor can
    // borrow the whole view.
    ui.push_id(("team", generation, team_id), |ui| {
        info_strip::view_title(ui, team_id, &team.name, TITLE_GAP);
        // The fields, the ids and the button on one row, since the Tactics
        // editor below needs the height, each unit one `allocate_ui` at its
        // claim. The pane is never narrower than the row (a narrower window
        // scales the whole UI down instead), so it never wraps.
        let mut open_kit_slots = false;
        ui.horizontal(|ui| {
            let row_gap = ui.spacing().item_spacing.x;
            for (unit, width) in TEAM_ROW {
                ui.add_space(id_cell_padding_before(unit, row_gap));
                ui.allocate_ui_with_layout(
                    egui::vec2(width, ui.spacing().interact_size.y),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = parts_gap(unit, row_gap);
                        let Some(session) = view.session.as_mut() else {
                            return;
                        };
                        match unit {
                            HeaderUnit::Name => name_pair(ui, session, team_id, version, &team),
                            HeaderUnit::Short => short_pair(ui, session, team_id, version, &team),
                            HeaderUnit::Colour(index) => {
                                colour_unit(ui, session, team_id, &team, index);
                            }
                            HeaderUnit::Manager => {
                                manager_unit(ui, session, team_id, schema, &team)
                            }
                            HeaderUnit::Stadium => {
                                stadium_unit(ui, session, team_id, schema, &team)
                            }
                            HeaderUnit::KitSlots => {
                                if ui
                                    .button(KIT_SLOTS_LABEL)
                                    .on_hover_text("The kit slot assignments, rarely edited")
                                    .clicked()
                                {
                                    open_kit_slots = true;
                                }
                            }
                        }
                    },
                );
            }
        });
        if open_kit_slots {
            view.kit_slots = Some(team_id);
        }
        // `item_spacing.y` trails the row and the line alike, so the
        // line's spacing is `LINE_BELOW` less one of them on each side,
        // plus the stroke the line paints at its centre, and the space
        // before it makes up `LINE_ABOVE`.
        let trailing = ui.spacing().item_spacing.y;
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke.width;
        ui.add_space(LINE_ABOVE - LINE_BELOW);
        ui.add(egui::Separator::default().spacing(2.0 * (LINE_BELOW - trailing) + stroke));
        tactics::show(ui, view);
    });
}

/// The Kit slot assignments dialog while `view.kit_slots` names its team
/// (the draft's `dlgKitSlots`): the assignments' rows in a modal, or the
/// sentence a version without them gets; OK or Escape closes.
pub(crate) fn kit_slots_dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some(team_id) = view.kit_slots else {
        return;
    };
    let Some(session) = view.session.as_mut() else {
        return;
    };
    let (version, team) = match session.file().team(team_id) {
        Some(team) => (session.file().version(), team.clone()),
        None => return,
    };
    let schema = schema_for(version);
    let mut close = false;
    // The rows' widget ids are the team's within this open, as the view's
    // are.
    let generation = view.generation;
    let id = egui::Id::unique("save_editor_kit_slots");
    let response = modal(ui.ctx(), id, "Kit slot assignments", |ui| {
        ui.push_id(("team", generation, team_id), |ui| {
            kit_slots(ui, session, team_id, schema, &team);
        });
        modal_buttons(ui, |ui| {
            if ui.button("OK").clicked() {
                close = true;
            }
        });
    });
    if close || response.should_close() {
        view.kit_slots = None;
    }
}

/// The name field's unit (the draft's `teamFields` pair). A keystroke
/// `team_text_fits` refuses is not taken — the field shows the stored text
/// again next frame, the info strip's Name field's rule. A changed name
/// sets the name flag.
fn name_pair(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    version: PesVersion,
    team: &TeamEntry,
) {
    ui.label(TEAM_LABEL);
    let mut name = team.name.clone();
    // `add_sized` allocates the width itself: the field keeps its own 140
    // (the draft's `#team-name`) whatever the wrapped row offers it.
    let response = ui.add_sized(
        [140.0, ui.spacing().interact_size.y],
        text_box(egui::TextEdit::singleline(&mut name).desired_width(140.0)),
    );
    if response.changed() && team_text_fits(version, TeamText::Name, &name) {
        edit_team(session, team_id, response.id.value(), |team| {
            team.name = name;
            team.edit_flags.name = true;
        });
    }
}

/// The short-name field's unit: upper-cased as typed and holding at most
/// three letters (the cap the draft's short-name field enforced; the
/// codec's four-byte field is unterminated and would take a fourth). A
/// change sets the name flag and, on PES 15, the short-name flag too.
fn short_pair(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    version: PesVersion,
    team: &TeamEntry,
) {
    ui.label(SHORT_LABEL);
    let mut short = team.short_name.clone();
    // The same `add_sized` allocation as the name field's above (the
    // draft's 40-point `#team-short`); `char_limit` is the draft's
    // `maxlength: 3`: a fourth character never enters the box.
    let response = ui.add_sized(
        [40.0, ui.spacing().interact_size.y],
        text_box(
            egui::TextEdit::singleline(&mut short)
                .desired_width(40.0)
                .char_limit(3),
        ),
    );
    if response.changed() {
        let upper = short.to_uppercase();
        if upper.chars().count() <= 3 && team_text_fits(version, TeamText::ShortName, &upper) {
            edit_team(session, team_id, response.id.value(), |team| {
                team.short_name = upper;
                team.edit_flags.name = true;
                if team.edit_flags.short_name.is_some() {
                    team.edit_flags.short_name = Some(true);
                }
            });
        }
    }
}

/// One team colour's unit (PES 17+): `color_tools`' picker with no
/// suggestions, the stored 6-bit channels shown as 8 bits and back by the
/// `view/` bullet's rule (`channel_to_show`/`channel_to_store`), the shown
/// colour's upper-case hex beside it. On PES 15/16 the pair draws disabled
/// with the `PES NN+` tooltip, in the same place.
fn colour_unit(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    team: &TeamEntry,
    index: usize,
) {
    ui.label(COLOUR_LABELS[index]);
    match &team.colors {
        Some(colors) => {
            let mut shown = [
                channel_to_show(colors[index].red),
                channel_to_show(colors[index].green),
                channel_to_show(colors[index].blue),
            ];
            let out = color_tools::picker::color_field(ui, ("team colour", index), &mut shown, &[]);
            if out.changed {
                let stored = TeamColor {
                    red: channel_to_store(shown[0]),
                    green: channel_to_store(shown[1]),
                    blue: channel_to_store(shown[2]),
                };
                edit_team(session, team_id, out.response.id.value(), |team| {
                    team.colors
                        .as_mut()
                        .expect("the field is drawn only on a version that stores it")[index] =
                        stored;
                });
            }
            // The shown colour is the nearest the save holds, so the hex
            // reads the rounded-back stored value, not the pick itself.
            let display = [
                channel_to_show(channel_to_store(shown[0])),
                channel_to_show(channel_to_store(shown[1])),
                channel_to_show(channel_to_store(shown[2])),
            ];
            ui.label(egui::RichText::new(format_hex(display)).monospace().weak());
        }
        None => {
            add_enabled_ui(ui, false, |ui| {
                color_tools::picker::color_field(ui, ("team colour", index), &mut [0, 0, 0], &[]);
            })
            .response
            .on_disabled_hover_text(gating_tooltip(
                [TeamField::Color1Red, TeamField::Color2Red][index],
            ));
            ui.weak(format_hex([0, 0, 0]));
        }
    }
}

/// The manager ID pair (PES 19+, `Option` on the entry). Elsewhere the
/// pair draws disabled with the `PES NN+` tooltip, in the same place.
fn manager_unit(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    schema: &VersionSchema,
    team: &TeamEntry,
) {
    ui.spacing_mut().interact_size.x = ID_BOX_WIDTH;
    match team.manager_id {
        Some(stored) => {
            number(
                ui,
                session,
                team_id,
                MANAGER_LABEL,
                stored,
                0..=u32::try_from(field_max(schema, TeamField::ManagerId))
                    .expect("the manager id's bound fits u32"),
                |team, shown| {
                    team.manager_id = Some(shown);
                },
            );
        }
        None => {
            ui.label(MANAGER_LABEL);
            gated_number(ui, TeamField::ManagerId);
        }
    }
}

/// The stadium ID pair (PES 19+, `Option` on the entry): a changed
/// stadium ID sets the stadium flag where the version has it (PES
/// 20/21). Elsewhere the pair draws disabled with the `PES NN+` tooltip,
/// in the same place.
fn stadium_unit(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    schema: &VersionSchema,
    team: &TeamEntry,
) {
    ui.spacing_mut().interact_size.x = ID_BOX_WIDTH;
    match team.stadium_id {
        Some(stored) => {
            number(
                ui,
                session,
                team_id,
                STADIUM_LABEL,
                stored,
                0..=u16::try_from(field_max(schema, TeamField::StadiumId))
                    .expect("the stadium id's bound fits u16"),
                |team, shown| {
                    team.stadium_id = Some(shown);
                    if team.edit_flags.stadium.is_some() {
                        team.edit_flags.stadium = Some(true);
                    }
                },
            );
        }
        None => {
            ui.label(STADIUM_LABEL);
            gated_number(ui, TeamField::StadiumId);
        }
    }
}

/// The ten kit slot rows (PES 17 only): each its Number (the stored `u8`)
/// and Bound team ID, drawn as `binding / 64` and written as
/// `id * 64 + binding % 64` (the low bits kept as stored). Either edit sets
/// the strip flag where the version has it (PES 17 alone, so `Some`), as
/// the stadium edit sets its own. Elsewhere the draft's no-records line,
/// its tooltip naming PES 17 only.
fn kit_slots(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    schema: &VersionSchema,
    team: &TeamEntry,
) {
    let Some(slots) = &team.kit_slots else {
        ui.weak("(this version does not store kit slots)")
            .on_hover_text("PES 17 only");
        return;
    };
    // The binding field's bit width gives the bound id's top (`binding` is
    // the id x 0x40, so the shown id runs to a sixty-fourth of the field's
    // own top).
    let bound_max = schema
        .team
        .max_value(TeamField::KitSlotTeam(0))
        .expect("the kit slots are drawn only where they are stored")
        / 64;
    egui::Grid::new("team kit slots")
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            for (index, kit) in slots.iter().enumerate() {
                ui.weak(format!("Slot {}", index + 1));
                number(
                    ui,
                    session,
                    team_id,
                    KIT_NUMBER_LABEL,
                    kit.number,
                    0..=u8::MAX,
                    move |team, shown| {
                        team.kit_slots
                            .as_mut()
                            .expect("the kit slots exist on PES 17")[index]
                            .number = shown;
                        if team.edit_flags.strip.is_some() {
                            team.edit_flags.strip = Some(true);
                        }
                    },
                );
                let low = kit.binding % 64;
                number(
                    ui,
                    session,
                    team_id,
                    BOUND_TEAM_LABEL,
                    kit.binding / 64,
                    0..=u32::try_from(bound_max).expect("the binding's bound fits u32"),
                    move |team, shown| {
                        team.kit_slots
                            .as_mut()
                            .expect("the kit slots exist on PES 17")[index]
                            .binding = shown * 64 + low;
                        if team.edit_flags.strip.is_some() {
                            team.edit_flags.strip = Some(true);
                        }
                    },
                );
                ui.end_row();
            }
        });
}

/// The biggest value `schema`'s team record stores in `field`
/// (`2^width - 1`, the codec's `ValueTooWide` bound): the range the view's
/// number controls take; called only where the entry holds `Some`.
fn field_max(schema: &VersionSchema, field: TeamField) -> u64 {
    schema
        .team
        .max_value(field)
        .expect("the field is stored by this version")
}

/// The shown 8-bit channel for a stored 6-bit one (`v * 255 / 63` rounded,
/// the widening of 5:6:5 colours the Team compiler uses).
pub(super) fn channel_to_show(v: u8) -> u8 {
    u8::try_from((u16::from(v) * 255 + 31) / 63).expect("63 maps to 255")
}

/// The stored 6-bit channel for a picked 8-bit one (`c * 63 / 255`
/// rounded): a pick always lands on a value the save holds.
fn channel_to_store(c: u8) -> u8 {
    u8::try_from((u16::from(c) * 63 + 127) / 255).expect("255 maps to 63")
}

/// `rgb` as `#RRGGBB` upper-case (the picker control's own hex form).
pub(super) fn format_hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// A `label`/`DragValue` pair over `shown` in `range`; on a change `store`
/// writes the shown value into the team's entry through `edit_team` (the
/// widget's id as the undo merge key). `clamp_existing_to_range` stays
/// off: a stored value past the range draws as stored, never clamped on
/// sight.
fn number<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    label: impl Into<egui::WidgetText>,
    shown: T,
    range: RangeInclusive<T>,
    store: impl FnOnce(&mut TeamEntry, T),
) {
    let mut shown = shown;
    ui.label(label);
    let mut response = ui.add(
        egui::DragValue::new(&mut shown)
            .range(range.clone())
            .clamp_existing_to_range(false),
    );
    if step_number(&mut shown, wheel_steps(ui, &response), range) {
        response.mark_changed();
    }
    if response.changed() {
        edit_team(session, team_id, response.id.value(), |team| {
            store(team, shown);
        });
    }
}

/// The disabled stand-in for a number the version does not store: `-`
/// inside the control's usual bounds (a Button at the DragValue's minimum
/// size, so an enabled version's control takes the same rect) with the
/// `PES NN+` tooltip (`abilities`' `gated_number` for team fields).
fn gated_number(ui: &mut egui::Ui, field: TeamField) {
    add_enabled(
        ui,
        false,
        egui::Button::new("-").min_size(ui.spacing().interact_size),
    )
    .on_disabled_hover_text(gating_tooltip(field));
}

/// The tooltip naming the first version that stores `field` (`PES NN+`):
/// `abilities`' `gating_tooltip` for team-record fields.
fn gating_tooltip(field: TeamField) -> String {
    format!(
        "PES {}+",
        first_version_with_team(field)
            .expect("a gated field is stored by some version")
            .number()
    )
}
