//! The Appearance screen (the draft's `appearanceTab` and `paletteRow`):
//! seven titled groups in `Frame::group` frames, laid out to show whole in
//! the default window — Full Player Customization with Colors at its
//! right, Strip style, Physique with Motion over Edit flags at its right,
//! and Presentation last (star and playing attitude, aesthetics like the
//! rest, so under the lock too). With the "Aesthetics from exports"
//! setting off the screen edits as every other does; with it on, the
//! ownership line and the "Edit aesthetics anyway" switch (`SaveView`'s
//! `edit_aesthetics`, off at every start of the tool) draw over the
//! groups, every control disabled under the ownership line as hover and
//! the switch re-enabling them for the rest of the run. Edits go through
//! `Session::edit_player` under the widget's id, one undo entry per
//! gesture, as `abilities.rs` does; a stored value past a control's range
//! draws as stored (`clamp_existing_to_range` off) and changes only when
//! edited.

pub(super) mod fields;

use pes_savefile::model::{
    PlayerAppearance, PlayerEditFlags, PlayerEntry, PlayerMotion, PlayerStats,
};
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::schema::ingame_face::IngameFaceField;
use pes_savefile::schema::limits::motion_top;
use pes_savefile::schema::{VersionSchema, player_max_value, schema_for};
use pes_savefile::settings_toml::{Kind, SettingKey};
use pes_version::PesVersion;
use studio_core::widgets::{
    FIELD_SPACING, add_enabled, add_enabled_ui, combo_item, number_arrows, number_box,
    number_wheel_steps, slider, step_choice, step_number, wheel_steps,
};

use crate::session::Session;
use crate::view::abilities::{
    self, gated_label, gated_number, gating_tooltip, maybe_end_row, number, small_stat,
};
use crate::view::appearance::fields::{
    MotionField, PHYSIQUE, Palette, STRIP_ENUMS, combo_options, motions, palette, strip_options,
};
use crate::view::group_frame::{GROUP_GAP, stretch_to};
use crate::view::team::SPACE_GAP;
use crate::view::{NUMBER_BOX, Panel, SaveView, edit_player, selected_player};

/// `edit_flags`' Face checkbox.
pub(super) const FACE_LABEL: &str = "Face";
/// `edit_flags`' Hair checkbox.
pub(super) const HAIR_LABEL: &str = "Hair";
/// `edit_flags`' Physique checkbox, which the group's heading shares.
pub(super) const PHYSIQUE_LABEL: &str = "Physique";
/// `edit_flags`' Strip checkbox.
pub(super) const STRIP_LABEL: &str = "Strip";
/// `edit_flags`' Motion checkbox, which the group's heading shares.
pub(super) const MOTION_LABEL: &str = "Motion";
/// `edit_flags`' Base copy checkbox.
pub(super) const BASE_COPY_LABEL: &str = "Base copy";
/// `edit_flags`' copy-from id number.
pub(super) const COPY_FROM_ID_LABEL: &str = "Copy from ID";
/// The strip grid's boots id number.
pub(super) const BOOTS_ID_LABEL: &str = "Boots ID";
/// The strip grid's gloves id number.
pub(super) const GLOVES_ID_LABEL: &str = "Gloves ID";
/// The strip grid's shirttail-out checkbox.
pub(super) const SHIRTTAIL_OUT_LABEL: &str = "Shirttail out";
/// The strip grid's ankle-taping checkbox.
pub(super) const ANKLE_TAPING_LABEL: &str = "Ankle taping";
/// The strip grid's wrist taping combo.
pub(super) const WRIST_TAPING_LABEL: &str = "Wrist taping";
/// The strip grid's left wrist-tape colour dropdown.
pub(super) const WRIST_TAPE_LEFT_LABEL: &str = "Wrist tape left";
/// The strip grid's right wrist-tape colour dropdown.
pub(super) const WRIST_TAPE_RIGHT_LABEL: &str = "Wrist tape right";
/// The strip grid's spectacles combo.
pub(super) const SPECTACLES_LABEL: &str = "Spectacles";
/// The strip grid's spectacles colour dropdown.
pub(super) const SPECTACLES_COLOR_LABEL: &str = "Spectacles color";
/// The strip grid's gloves colour dropdown.
pub(super) const GLOVES_COLOR_LABEL: &str = "Gloves color";
/// The strip grid's outfielder-gloves dropdown.
pub(super) const OUTFIELDER_GLOVES_LABEL: &str = "Outfielder gloves";
/// The outfielder-gloves dropdown's two readings — `gloves == 0` and any
/// non-zero — in the combo's pick order.
pub(super) const GLOVES_OPTIONS: [&str; 2] = ["Off", "Worn in winter"];

/// `version`'s outfielder-gloves pair (`Off` first): "Auto" on PES 2014,
/// where the bit sits in the appearance record; "Worn in winter" where it
/// lives in the in-game face run.
pub(super) fn gloves_options(version: PesVersion) -> [&'static str; 2] {
    if version == PesVersion::Pes14 {
        ["Off", "Auto"]
    } else {
        GLOVES_OPTIONS
    }
}

/// The dribbling motion's number.
pub(super) const DRIBBLING_MOTION_LABEL: &str = "Dribbling motion";
/// The Presentation group's title.
const PRESENTATION_LABEL: &str = "Presentation";
/// The star box's note.
const STAR_NOTE: &str =
    "How likely the player is to show up as the team's featured player in the pre-match screen.";
/// The playing attitude box's note.
const ATTITUDE_NOTE: &str = "How temperamental the player is in cutscenes.";

/// Between two groups side by side (the draft's `.aprow` `gap`).
const SIDE_GAP: f32 = 8.0;
/// A strip cell's label slot (the draft's `.slabel`).
const LABEL_W: f32 = 104.0;
/// Between a strip cell's label and its field (the draft's `.scell` `gap`).
const LABEL_FIELD_GAP: f32 = 8.0;
/// A strip cell's field slot: the dropdowns, the two ID boxes and the
/// checkboxes' cells (the draft's `.scell` inputs).
const FIELD_W: f32 = 118.0;
/// Between a strip cell's field and the next cell's label (the draft's
/// `.scell` padding, 14 on each side).
const CELL_GAP: f32 = 28.0;
/// One strip column's pitch: its cell and the gap after it.
const STRIP_PITCH: f32 = LABEL_W + LABEL_FIELD_GAP + FIELD_W + CELL_GAP;
/// A strip grid row's height (the draft's `.scell` `height`).
const STRIP_ROW_H: f32 = 24.0;
/// A physique row's height: the rows a little further apart, so the group
/// fills to the right column's height (the draft's physique `td` padding).
const PHYSIQUE_ROW_H: f32 = 34.0;
/// A physique slider's rail length (the draft's `input[type=range]` 82).
const PHYSIQUE_RAIL_W: f32 = 82.0;
/// An edit-flags cell's height (the draft's `#edit-flags-grid td`).
const FLAG_CELL_H: f32 = 22.0;
/// Between an edit-flags row's cells (the draft's `td` padding-right).
const FLAG_CELL_GAP: f32 = 12.0;
/// Between the Colors rows: their 14-point swatches, on 18-point rows, stand
/// 21 apart as the draft's do, 7 points between them.
const SWATCH_ROW_SPACING: f32 = 3.0;
/// Between two squares of a Colors row (the draft's `.palette` `gap`).
const SWATCH_GAP: f32 = 2.0;
/// The padding inside a swatch row's cell (the draft's `td`, a side).
const SWATCH_CELL_PAD: f32 = 4.0;

/// The Appearance screen's content for the selected player.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some((_, _, player_id)) = selected_player(view) else {
        return;
    };
    // The setting decides the lock's draw (`aesthetics_locked`): with it
    // off there is no ownership line and no switch and the groups edit;
    // with it on the ownership line and the session's switch.
    let (setting, locked) = (view.aesthetics_from_exports, view.aesthetics_locked());
    let SaveView {
        session,
        edit_aesthetics,
        randomize_draw,
        generation,
        hide_ids,
        ..
    } = view;
    let generation = *generation;
    let Some(session) = session.as_mut() else {
        return;
    };
    let (version, player) = {
        let file = session.file();
        let Some(player) = file.player(player_id) else {
            return;
        };
        (file.version(), player.clone())
    };
    // Every widget id under the section is the player's within this open,
    // its own salt since it draws beside the Abilities & Skills content
    // (whose `push_id` salts the same pair): two scopes with one salt would
    // hand two widgets one id — an in-progress edit and its undo key would
    // cross between the sections.
    let mut open_patch = false;
    ui.push_id(("appearance", generation, player_id), |ui| {
        if setting {
            open_patch = ownership_line(ui);
            studio_core::widgets::checkbox(ui, edit_aesthetics, "Edit aesthetics anyway");
            if *edit_aesthetics {
                ui.weak("A later patch overwrites whatever is edited by hand.");
            }
        }
        add_enabled_ui(ui, !locked, |ui| {
            groups(
                ui,
                session,
                player_id,
                version,
                &player,
                randomize_draw.as_mut(),
                *hide_ids,
            );
        })
        .response
        .on_disabled_hover_text(OWNERSHIP_TIP);
    });
    if open_patch {
        view.panel = Some(Panel::Patch);
    }
}

/// The screen's groups, `GROUP_GAP` apart: Full Player Customization with
/// Colors at its right, Strip style the pane's width, Physique with
/// Motion over Edit flags at its right, and Presentation the pane's
/// width, last.
fn groups(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    player: &PlayerEntry,
    draw: &mut dyn FnMut(u8) -> u8,
    hide_ids: Result<fpc::HideIds, fpc::HideIdsError>,
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    // A control whose field the version's records do not map draws greyed:
    // the schema's `player_has` gates the record fields, `face_run` the
    // in-game face run's.
    let schema = schema_for(version);
    let face_run = schema.has_ingame_face();
    // PES 2014 holds an appearance record for an edited or created
    // player alone: one lookup per draw decides whether the record's own
    // fields edit or grey under `NO_LOOK_TIP` (every other version holds
    // one per player).
    let has_record = session.has_appearance_record(player_id);
    // `GROUP_GAP` stands the first group off the title's line as it
    // stands each group off the last.
    ui.add_space(GROUP_GAP);
    fpc_colors_row(ui, session, player_id, version, player, hide_ids, face_run);
    ui.add_space(GROUP_GAP);
    group(ui, "Strip style", |ui| {
        ui.set_min_width(ui.available_width());
        strip_grid(
            ui,
            session,
            player_id,
            version,
            &player.appearance,
            &StripGates {
                schema,
                face_run,
                has_record,
            },
        );
    });
    ui.add_space(GROUP_GAP);
    physique_motion_flags_row(ui, session, player_id, player, draw, schema, has_record);
    ui.add_space(GROUP_GAP);
    group(ui, PRESENTATION_LABEL, |ui| {
        ui.set_min_width(ui.available_width());
        presentation(ui, session, player_id, &player.stats);
    });
}

/// One titled group (the draft's `.group`): egui's group frame, the title
/// in the strong colour and `TITLE_GAP` above the first thing drawn under
/// it, as the Stats screen's panels leave under theirs, the content
/// through the Stats screen's `inset`.
fn group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.strong(title);
        ui.add_space(abilities::TITLE_GAP - ui.spacing().item_spacing.y);
        abilities::inset(ui, add);
    });
}

/// `prepared`'s `content_ui` grown to `height` before its frame paints:
/// `stretch_to` for the height alone.
fn stretch(prepared: &mut egui::containers::frame::Prepared, height: f32) {
    stretch_to(
        prepared,
        egui::vec2(prepared.content_ui.min_rect().width(), height),
    );
}

/// A group frame's inside for the rows: a top-down scope from the frame's
/// left (a right-to-left row's frame would inherit the row's layout),
/// `title` in the strong colour and `TITLE_GAP` under it, as the Stats
/// screen's panels leave under theirs, the content through their `inset`.
fn frame_content(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.scope_builder(
        egui::UiBuilder::new().layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.strong(title);
            ui.add_space(abilities::TITLE_GAP - ui.spacing().item_spacing.y);
            abilities::inset(ui, add);
        },
    );
}

/// The egui memory id `fpc_colors_row` writes the Colors group's measured
/// outer width to each pass (FPC's width reads the previous pass's; a pass
/// that measures a different one is discarded).
const COLORS_WIDTH_ID: &str = "save_editor appearance colors width";

/// The first row: Full Player Customization filling the room Colors
/// leaves (the draft's `.aprow.fpcrow`: FPC flexes, Colors keeps its own
/// width), `SIDE_GAP` between them and the two frames' bottoms level —
/// the shorter one's frame repainted to the row's height.
fn fpc_colors_row(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    player: &PlayerEntry,
    hide_ids: Result<fpc::HideIds, fpc::HideIdsError>,
    face_run: bool,
) {
    let group_frame = egui::Frame::group(ui.style());
    let width_id = egui::Id::unique(COLORS_WIDTH_ID);
    let measured: Option<f32> = ui.ctx().data(|data| data.get_temp(width_id));
    // Left to right, FPC first at the width the last pass's Colors leaves.
    // Not a right-to-left row adding Colors first: a frame's content in it
    // spans the whole row, and a right-aligned top-down content puts its
    // grid's cursor at -inf (`Negative desired size: [NaN 18.0]`).
    ui.scope_builder(
        egui::UiBuilder::new().layout(egui::Layout::left_to_right(egui::Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing.x = SIDE_GAP;
            let fpc_width = ui.available_width() - SIDE_GAP - measured.unwrap_or(0.0);
            let mut fpc = group_frame.begin(ui);
            fpc.content_ui
                .set_min_width(fpc_width - group_frame.total_margin().sum().x);
            frame_content(&mut fpc.content_ui, "Full Player Customization", |ui| {
                fpc_group(ui, session, player_id, player, hide_ids, face_run);
            });
            fpc.allocate_space(ui);
            let mut colors = group_frame.begin(ui);
            frame_content(&mut colors.content_ui, "Colors", |ui| {
                colour_rows(
                    ui,
                    session,
                    player_id,
                    version,
                    &player.appearance,
                    face_run,
                );
            });
            let width = colors.allocate_space(ui).rect.width();
            ui.ctx().data_mut(|data| data.insert_temp(width_id, width));
            if measured != Some(width) {
                ui.ctx()
                    .request_discard("the Appearance screen's Colors width changed");
            }
            let height = fpc
                .content_ui
                .min_rect()
                .height()
                .max(colors.content_ui.min_rect().height());
            stretch(&mut fpc, height);
            fpc.paint(ui);
            stretch(&mut colors, height);
            colors.paint(ui);
        },
    );
}

/// The second row: Physique at its own width, and beside it a column
/// taking the rest — Motion over Edit flags, `GROUP_GAP` between them —
/// the two sides ending at one bottom (the draft's `.aprow` stretch: the
/// shorter side's frame repainted down to the taller side's bottom, Edit
/// flags for the column).
fn physique_motion_flags_row(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    player: &PlayerEntry,
    draw: &mut dyn FnMut(u8) -> u8,
    schema: &VersionSchema,
    has_record: bool,
) {
    let group_frame = egui::Frame::group(ui.style());
    let margin = group_frame.total_margin().sum().y;
    ui.scope_builder(
        egui::UiBuilder::new().layout(egui::Layout::left_to_right(egui::Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing.x = SIDE_GAP;
            let mut physique = group_frame.begin(ui);
            frame_content(&mut physique.content_ui, PHYSIQUE_LABEL, |ui| {
                physique_grid(
                    ui,
                    session,
                    player_id,
                    &player.appearance,
                    schema,
                    has_record,
                )
            });
            physique.allocate_space(ui);
            let physique_outer = physique.content_ui.min_rect().height() + margin;
            let row_height = ui
                .scope_builder(
                    egui::UiBuilder::new().layout(egui::Layout::top_down(egui::Align::Min)),
                    |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = GROUP_GAP;
                        let mut motion = group_frame.begin(ui);
                        frame_content(&mut motion.content_ui, MOTION_LABEL, |ui| {
                            ui.set_min_width(ui.available_width());
                            motion_grid(
                                ui,
                                session,
                                player_id,
                                schema.version,
                                &player.motion,
                                draw,
                                schema,
                            );
                        });
                        let motion_outer = motion.content_ui.min_rect().height() + margin;
                        motion.end(ui);
                        let mut flags = group_frame.begin(ui);
                        frame_content(&mut flags.content_ui, "Edit flags", |ui| {
                            ui.set_min_width(ui.available_width());
                            edit_flags_grid(
                                ui,
                                session,
                                player_id,
                                &player.edit_flags,
                                player.appearance.base_copy_id,
                                schema,
                                has_record,
                            );
                        });
                        flags.allocate_space(ui);
                        let column_height = ui.min_rect().height();
                        let row_height = physique_outer.max(column_height);
                        if column_height < row_height {
                            stretch(&mut flags, row_height - motion_outer - GROUP_GAP - margin);
                            let column = ui.min_rect();
                            ui.advance_cursor_after_rect(egui::Rect::from_min_max(
                                column.min,
                                egui::pos2(column.max.x, column.min.y + row_height),
                            ));
                        }
                        flags.paint(ui);
                        row_height
                    },
                )
                .inner;
            stretch(&mut physique, row_height - margin);
            physique.paint(ui);
        },
    );
}

/// The ownership row: the sentence itself and the Apply aesthetics patch
/// link. Whether the link was clicked: the caller opens the patch panel.
fn ownership_line(ui: &mut egui::Ui) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.label(OWNERSHIP_TIP);
        // The link is never broken across two rows: when it does not fit
        // after the sentence, it starts the next row whole.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.link("Apply aesthetics patch").clicked()
    })
    .inner
}

/// The ownership sentence (the draft's `ownershipTip`): the line over the
/// section, the disabled groups' hover and the locked Team-menu items'
/// (`menus.rs`).
pub(crate) const OWNERSHIP_TIP: &str = "The team's export owns the aesthetics, so edit its \
                                        settings.toml (or its ingame_face.toml, for the face, \
                                        hair and iris), recompile, then apply the patch.";

/// The disabled hover of a control a PES 2014 save never offers: the cells
/// of the in-game face run (whose records the version maps none of) and the
/// menus' out-of-scope items, which share it (`menus.rs`).
pub(crate) const PES14_TIP: &str = "Not available on PES 2014 saves";

/// The disabled hover of a look control on a PES 2014 player the save
/// holds no appearance record for (the version stores one for an edited
/// or created player alone): the game fills a record in on an in-game
/// edit; the editor never makes one.
pub(crate) const NO_LOOK_TIP: &str =
    "PES 2014 stores no look for this player yet: edit it in game first";

/// The FPC group, the first row's left side (the draft's `fpcGroup`): the
/// classifier's reading (`is_fpc_player` under `hide_ids`, the settings'
/// hide IDs — a player hidden by his model's own boots ID reads as shown,
/// so no checkbox), the Hide default body and Show default body buttons
/// with the draft's hover texts, then the interference findings of
/// `fpc::check` in its order. Under the lock the whole group is disabled
/// with the rest of the section — the reading and the findings still draw,
/// since they read and do not write. `hide_ids` an `Err` (a setting outside
/// its band): the line reads the error's text instead of the default-body
/// reading and both buttons are disabled with it as their hover; the
/// classifier has no IDs to read, so no findings either. On a version
/// whose records carry no in-game face run the reading, the buttons and
/// the findings all go through a run that is not there: the group is the
/// two buttons, disabled with `PES14_TIP`.
fn fpc_group(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    player: &PlayerEntry,
    hide_ids: Result<fpc::HideIds, fpc::HideIdsError>,
    face_run: bool,
) {
    if !face_run {
        ui.horizontal_wrapped(|ui| {
            for label in ["Hide default body", "Show default body"] {
                add_enabled(ui, false, egui::Button::new(label)).on_disabled_hover_text(PES14_TIP);
            }
        });
        return;
    }
    let ids = match hide_ids {
        Ok(ids) => ids,
        Err(error) => {
            ui.horizontal_wrapped(|ui| {
                ui.label(error.to_string());
                for label in ["Hide default body", "Show default body"] {
                    add_enabled(ui, false, egui::Button::new(label))
                        .on_disabled_hover_text(error.to_string());
                }
            });
            return;
        }
    };
    let hidden = pes_savefile::ops::fpc::is_fpc_player(player, session.version(), ids)
        .expect("a loaded player's face run holds the skin field");
    ui.horizontal_wrapped(|ui| {
        ui.label(if hidden {
            "The default body is currently hidden"
        } else {
            "The default body is currently shown"
        });
        if ui
            .button("Hide default body")
            .on_hover_text(
                "Write long sleeves, a tucked shirt and short socks. \
                 A boots or gloves ID of 0 takes the hide ID from Settings (a model's \
                 own ID is kept), and a Custom skin (PES 2015 to 2017) becomes Light. \
                 Ctrl+I switches this player between hidden and shown, and the Team \
                 menu hides or shows the whole team.",
            )
            .clicked()
        {
            session.apply_fpc(player_id, fpc::Preset::Hide, ids);
        }
        if ui
            .button("Show default body")
            .on_hover_text(
                "Write short sleeves, an untucked shirt and standard \
                 socks. A boots or gloves ID that holds the hide ID goes back to 0, and \
                 a Custom skin (PES 2015 to 2017) becomes Light.",
            )
            .clicked()
        {
            session.apply_fpc(player_id, fpc::Preset::Unhide, ids);
        }
    });
    let style = pes_savefile::ops::fpc::strip_style(player)
        .expect("a loaded player's face run holds the strip fields");
    let findings = fpc::check(&style, hidden);
    if findings.is_empty() {
        ui.weak("No interference found.");
    } else {
        for finding in &findings {
            let message = crate::messages::fpc_finding(finding);
            let text = format!(
                "{}: {}",
                message.severity.name(),
                crate::messages::when_text(&message)
            );
            match message.severity {
                studio_core::Severity::Error | studio_core::Severity::Fatal => {
                    ui.colored_label(ui.visuals().error_fg_color, text);
                }
                studio_core::Severity::Warning => {
                    ui.colored_label(ui.visuals().warn_fg_color, text);
                }
                studio_core::Severity::Info => {
                    ui.label(text);
                }
            }
        }
    }
}

/// The Presentation group's two rows: star and playing attitude one under
/// the other, label then box, each with its note at the box's right in the
/// weak colour. `PRESENTATION_STATS` because the pair shares the Profile
/// fields' `SmallStat` shape.
fn presentation(ui: &mut egui::Ui, session: &mut Session, player_id: u32, stats: &PlayerStats) {
    // The boxes at the draft's number-input width; no column floor, so
    // the label column keeps its labels' width.
    egui::Grid::new("appearance presentation")
        .num_columns(3)
        .min_col_width(0.0)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FIELD_SPACING))
        .show(ui, |ui| {
            ui.spacing_mut().interact_size.x = NUMBER_BOX;
            for (small, note) in fields::PRESENTATION_STATS
                .iter()
                .zip([STAR_NOTE, ATTITUDE_NOTE])
            {
                small_stat(ui, session, player_id, stats, small);
                ui.weak(note);
                ui.end_row();
            }
        });
}

/// A record field's gate for the selected player: the version stores it
/// (`player_has`, else the `PES NN+` stand-in) and, on PES 2014, the
/// player has the appearance record the field's run sits in (else the
/// `NO_LOOK_TIP` stand-in — the record holds the look of an edited or
/// created player alone). A field of the player record, a motion's say,
/// is never record-gated.
fn field_gate(schema: &VersionSchema, has_record: bool, field: PlayerField) -> (bool, String) {
    if !schema.player_has(field) {
        return (false, gating_tooltip(field));
    }
    if !has_record && schema.appearance_only(field) {
        return (false, NO_LOOK_TIP.to_owned());
    }
    (true, String::new())
}

/// The edit flags' grid (the draft's `#edit-flags-grid`): the six flags
/// three a row, `Copy from ID` on a row of its own under them. `base_copy`
/// exists on PES 15 to 18 only; like a gated field it keeps its place,
/// disabled. Ticking it also sets the seven gameplay flags, since a copy's
/// sections count as edited; unticking clears `base_copy` alone.
fn edit_flags_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    flags: &PlayerEditFlags,
    base_copy_id: u32,
    schema: &VersionSchema,
    has_record: bool,
) {
    egui::Grid::new("appearance edit flags")
        .num_columns(3)
        .min_row_height(FLAG_CELL_H)
        .spacing(egui::vec2(FLAG_CELL_GAP, 0.0))
        .show(ui, |ui| {
            for (index, &(field, label, get, set)) in FLAG_CELLS.iter().enumerate() {
                // A flag the version stores edits; elsewhere the box keeps
                // its place, disabled as `base_copy`'s is.
                let (enabled, tip) = field_gate(schema, has_record, field);
                if enabled {
                    checkbox(ui, session, player_id, label, get(flags), |p, v| {
                        set(&mut p.edit_flags, v)
                    });
                } else {
                    add_enabled(ui, false, |ui: &mut egui::Ui| {
                        studio_core::widgets::checkbox(ui, &mut false, label)
                    })
                    .on_disabled_hover_text(tip);
                }
                if index == 2 {
                    ui.end_row();
                }
            }
            match flags.base_copy {
                Some(held) => checkbox(ui, session, player_id, BASE_COPY_LABEL, held, |p, v| {
                    p.edit_flags.base_copy = Some(v);
                    if v {
                        p.edit_flags.abilities = true;
                        p.edit_flags.com_styles = true;
                        p.edit_flags.motion = true;
                        p.edit_flags.playable_positions = true;
                        p.edit_flags.registered_position = true;
                        p.edit_flags.skills = true;
                        p.edit_flags.playing_style = true;
                    }
                }),
                None => {
                    add_enabled(ui, false, |ui: &mut egui::Ui| {
                        studio_core::widgets::checkbox(ui, &mut false, BASE_COPY_LABEL)
                    })
                    .on_disabled_hover_text("PES 15 to 18 only");
                }
            }
            ui.end_row();
        });
    // Copy from ID's box stands as if centred on a `FLAG_CELL_H` row of its own
    // under the flags, as the draft's grid row draws it. The draft's label
    // and box share the cell: a space parts them, and the box is the number
    // input's width.
    ui.add_space((FLAG_CELL_H - ui.spacing().interact_size.y) / 2.0 - ui.spacing().item_spacing.y);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACE_GAP;
        ui.spacing_mut().interact_size.x = NUMBER_BOX;
        if schema.player_has(PlayerField::BaseCopyId) {
            number(
                ui,
                session,
                player_id,
                COPY_FROM_ID_LABEL,
                base_copy_id,
                0..=u32::MAX,
                |player, shown| {
                    player.appearance.base_copy_id = shown;
                },
            );
        } else {
            gated_label(
                ui,
                COPY_FROM_ID_LABEL,
                gating_tooltip(PlayerField::BaseCopyId),
            );
            gated_number(ui, gating_tooltip(PlayerField::BaseCopyId), NUMBER_BOX);
        }
    });
}

/// One flag cell of the edit-flags grid: the schema field the box gates on,
/// its label, and the `PlayerEditFlags` accessors.
type FlagCell = (
    PlayerField,
    &'static str,
    fn(&PlayerEditFlags) -> bool,
    fn(&mut PlayerEditFlags, bool),
);

/// The five plain flag cells in the draft's grid order: face, hair,
/// physique, strip, motion (`base_copy` keeps its `Option` arm below).
const FLAG_CELLS: [FlagCell; 5] = [
    (
        PlayerField::EditedFace,
        FACE_LABEL,
        |f| f.face,
        |f, v| f.face = v,
    ),
    (
        PlayerField::EditedHair,
        HAIR_LABEL,
        |f| f.hair,
        |f, v| f.hair = v,
    ),
    (
        PlayerField::EditedPhysique,
        PHYSIQUE_LABEL,
        |f| f.physique,
        |f, v| f.physique = v,
    ),
    (
        PlayerField::EditedStrip,
        STRIP_LABEL,
        |f| f.strip,
        |f, v| f.strip = v,
    ),
    (
        PlayerField::EditedMotion,
        MOTION_LABEL,
        |f| f.motion,
        |f, v| f.motion = v,
    ),
];

/// The 14 physique fields in an aligned grid of label and slider cells,
/// two pairs a row on `PHYSIQUE_ROW_H`-tall rows, each slider's rail
/// `PHYSIQUE_RAIL_W` long with its value box at the right (the draft's
/// `.stats-grid` pairs and 82-point sliders). The arrow keys step the
/// value box by one, as the suite's other number boxes do. Each field
/// is on its key's `Signed` scale (±7: `shown - limit` stores `shown +
/// limit`). The cells are padded `CELL_PADDING` a side, as the draft's
/// `.stats-grid` cells are: twice that between two cells and once after
/// the last box (the group's inset is the one before the first label).
fn physique_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    appearance: &PlayerAppearance,
    schema: &VersionSchema,
    has_record: bool,
) {
    let padding = abilities::CELL_PADDING;
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            right: padding,
            ..egui::Margin::ZERO
        })
        .show(ui, |ui| {
            egui::Grid::new("appearance physique")
                .num_columns(4)
                .min_row_height(PHYSIQUE_ROW_H)
                .spacing(egui::vec2(2.0 * f32::from(padding), 0.0))
                .show(ui, |ui| {
                    for (index, (label, key, get, set)) in PHYSIQUE.iter().enumerate() {
                        let field = key
                            .player_field()
                            .expect("a physique key writes a player field");
                        let (enabled, tip) = field_gate(schema, has_record, field);
                        if !enabled {
                            gated_label(ui, label, tip.clone());
                            // The enabled control is a slider: its rail,
                            // the gap and its value box.
                            gated_number(
                                ui,
                                tip,
                                PHYSIQUE_RAIL_W
                                    + ui.spacing().item_spacing.x
                                    + ui.spacing().interact_size.x,
                            );
                            maybe_end_row(ui, index + 1, 2);
                            continue;
                        }
                        let Kind::Signed { limit } = key.spec().kind else {
                            unreachable!("the physique keys are Signed");
                        };
                        let limit = i32::from(limit);
                        let held = get(appearance);
                        let mut shown = i32::from(held) - limit;
                        ui.label(*label);
                        let wheel_id = ui.scope_id().with(*label);
                        let response =
                            number_arrows(ui, &mut shown, -limit..=limit, |ui, value| {
                                slider(ui, value, -limit..=limit, PHYSIQUE_RAIL_W, wheel_id)
                            });
                        if response.changed() {
                            edit_player(session, player_id, response.id.value(), |player| {
                                set(
                                    &mut player.appearance,
                                    u8::try_from(shown + limit).expect(
                                        "the control keeps the value inside the key's range",
                                    ),
                                );
                            });
                        }
                        maybe_end_row(ui, index + 1, 2);
                    }
                });
        });
}

/// One strip-grid cell (the draft's `stripCells` entries): a `STRIP_ENUMS`
/// tuple's own kind, or one of the five kinds the grid holds besides
/// (the two ID boxes, the colour dropdowns, the outfielder-gloves
/// dropdown and the two checkboxes).
#[derive(Clone, Copy)]
enum StripCell {
    /// The boots ID box, on the record's own bound (`id_box`).
    BootsId,
    /// The same for the gloves ID, with the any-player hover.
    GlovesId,
    /// A `STRIP_ENUMS` entry (`label`, `SettingKey`, `get`, `set`).
    Enum(&'static fields::AppearanceField),
    /// The wrist taping dropdown.
    WristTaping,
    /// The left wrist tape's colour dropdown (`combo_options`' labels).
    WristTapeLeft,
    /// The right wrist tape's colour dropdown.
    WristTapeRight,
    /// `Off`/`Worn in winter` over the face run's PlayerGloves (not a
    /// checkbox: beside the colours it read as one of them, and it did not
    /// say when the gloves show).
    OutfielderGloves,
    /// The gloves colour dropdown over the face run's PlayerGlovesColor.
    GlovesColor,
    /// The ankle-taping checkbox, a bare box in the field slot.
    AnkleTaping,
    /// The spectacles style dropdown.
    Spectacles,
    /// The spectacles colour dropdown.
    SpectaclesColor,
    /// The shirttail-out checkbox, a bare box in the field slot.
    ShirttailOut,
}

/// The fifteen cells in the draft's order, three columns by five
/// rows (the draft's `STRIP_ORDER`).
const STRIP_ORDER: [[StripCell; 3]; 5] = [
    [
        StripCell::BootsId,
        StripCell::Enum(&STRIP_ENUMS[0]),
        StripCell::Enum(&STRIP_ENUMS[1]),
    ],
    [
        StripCell::GlovesId,
        StripCell::Enum(&STRIP_ENUMS[2]),
        StripCell::Enum(&STRIP_ENUMS[3]),
    ],
    [
        StripCell::WristTaping,
        StripCell::WristTapeLeft,
        StripCell::WristTapeRight,
    ],
    [
        StripCell::OutfielderGloves,
        StripCell::GlovesColor,
        StripCell::AnkleTaping,
    ],
    [
        StripCell::Spectacles,
        StripCell::SpectaclesColor,
        StripCell::ShirttailOut,
    ],
];

/// The strip cells' three gates: the fields `version`'s records store,
/// whether they carry the in-game face run the two gloves cells write,
/// and whether the player has the appearance record the record's own
/// fields sit in (every player's on the other versions; PES 2014's
/// edited or created players' alone).
struct StripGates<'a> {
    /// The version's schema (`player_has` gates each cell's field).
    schema: &'a VersionSchema,
    /// Whether the records carry an in-game face run (`has_ingame_face`).
    face_run: bool,
    /// Whether the player has an appearance record (`has_appearance_record`).
    has_record: bool,
}

/// `cell`'s gate: the `PlayerField` it writes, or `None` for the cells of
/// the in-game face run, whose gate is the run itself. PES 2014 keeps its
/// outfielder gloves in the appearance record instead, under its own
/// field.
fn cell_field(cell: StripCell, version: PesVersion) -> Option<PlayerField> {
    match cell {
        StripCell::BootsId => Some(PlayerField::BootsId),
        StripCell::GlovesId => Some(PlayerField::GlovesId),
        StripCell::Enum(&(_, key, _, _)) => Some(
            key.player_field()
                .expect("a strip cell's key writes a player field"),
        ),
        StripCell::WristTaping => Some(PlayerField::WristTaping),
        StripCell::WristTapeLeft => Some(PlayerField::WristTapeColorLeft),
        StripCell::WristTapeRight => Some(PlayerField::WristTapeColorRight),
        StripCell::OutfielderGloves if version == PesVersion::Pes14 => {
            Some(PlayerField::PlayerGloves)
        }
        StripCell::OutfielderGloves | StripCell::GlovesColor => None,
        StripCell::AnkleTaping => Some(PlayerField::AnkleTaping),
        StripCell::Spectacles => Some(PlayerField::SpectaclesStyle),
        StripCell::SpectaclesColor => Some(PlayerField::SpectaclesColor),
        StripCell::ShirttailOut => Some(PlayerField::Untucked),
    }
}

/// `cell`'s label, drawn in its `LABEL_W` slot.
fn cell_label(cell: StripCell) -> &'static str {
    match cell {
        StripCell::BootsId => BOOTS_ID_LABEL,
        StripCell::GlovesId => GLOVES_ID_LABEL,
        StripCell::Enum(&(label, ..)) => label,
        StripCell::WristTaping => WRIST_TAPING_LABEL,
        StripCell::WristTapeLeft => WRIST_TAPE_LEFT_LABEL,
        StripCell::WristTapeRight => WRIST_TAPE_RIGHT_LABEL,
        StripCell::OutfielderGloves => OUTFIELDER_GLOVES_LABEL,
        StripCell::GlovesColor => GLOVES_COLOR_LABEL,
        StripCell::AnkleTaping => ANKLE_TAPING_LABEL,
        StripCell::Spectacles => SPECTACLES_LABEL,
        StripCell::SpectaclesColor => SPECTACLES_COLOR_LABEL,
        StripCell::ShirttailOut => SHIRTTAIL_OUT_LABEL,
    }
}

/// The Strip style group's grid (the draft's `#strip-grid`): the fifteen
/// settings three columns by five rows, each cell `LABEL_W` for its label
/// then `FIELD_W` for its field, `CELL_GAP` between a field and the next
/// label, rows `STRIP_ROW_H` tall and touching, with the thin lines in
/// the group frame's stroke colour parting what belongs together — under
/// the second and third rows across the three columns, under the fourth
/// across the first two, between the columns on the first two rows and
/// at the third column's left on the last two (the draft's `lr`/`lb`
/// cells).
fn strip_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
    gates: &StripGates,
) {
    let mut rows = Vec::with_capacity(STRIP_ORDER.len());
    ui.spacing_mut().item_spacing.y = 0.0;
    for row in STRIP_ORDER {
        let inner = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.set_min_height(STRIP_ROW_H);
            for cell in row {
                strip_cell(ui, session, player_id, version, appearance, cell, gates);
            }
        });
        rows.push(inner.response.rect);
    }
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    let painter = ui.painter();
    let left = rows[0].min.x;
    // The lines under rows two, three (the three columns) and four (the
    // first two), at the touched rows' shared bottom, ending mid-gap where
    // a column line stands. They start under the group's title, the
    // group's inset before the first labels, as the draft's first column's
    // cells take the inset as their own padding.
    let lines_left = left - f32::from(abilities::CELL_PADDING);
    for (under, columns) in [(1usize, 3usize), (2, 3), (3, 2)] {
        let y = rows[under].max.y;
        painter.line_segment(
            [
                egui::pos2(lines_left, y),
                egui::pos2(left + columns as f32 * STRIP_PITCH - CELL_GAP / 2.0, y),
            ],
            stroke,
        );
    }
    // The column lines, centred in the `CELL_GAP` between a field and the
    // next label: between the columns on rows one and two, at the third
    // column's left on rows four and five.
    for (column, first_row, last_row) in [(1usize, 0usize, 1usize), (2, 0, 1), (2, 3, 4)] {
        let x = left + column as f32 * STRIP_PITCH - CELL_GAP / 2.0;
        painter.line_segment(
            [
                egui::pos2(x, rows[first_row].min.y),
                egui::pos2(x, rows[last_row].max.y),
            ],
            stroke,
        );
    }
}

/// One strip cell: `cell`'s label in its `LABEL_W` slot, `LABEL_FIELD_GAP`,
/// then its field in the `FIELD_W` slot, `CELL_GAP` after it. A cell whose
/// field the version's records do not store — or whose in-game face run is
/// not there — draws the gated stand-in instead, label dimmed with it.
fn strip_cell(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
    cell: StripCell,
    gates: &StripGates,
) {
    // The gated stand-in's tooltip is computed only where it shows:
    // `gating_tooltip` panics on a field no version stores.
    let (enabled, tip) = match cell_field(cell, version) {
        Some(field) => field_gate(gates.schema, gates.has_record, field),
        None => (gates.face_run, PES14_TIP.to_owned()),
    };
    ui.allocate_ui_with_layout(
        egui::vec2(LABEL_W, STRIP_ROW_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(egui::vec2(LABEL_W, STRIP_ROW_H));
            if enabled {
                ui.label(cell_label(cell));
            } else {
                gated_label(ui, cell_label(cell), tip.clone());
            }
        },
    );
    ui.add_space(LABEL_FIELD_GAP);
    ui.allocate_ui_with_layout(
        egui::vec2(FIELD_W, STRIP_ROW_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(egui::vec2(FIELD_W, STRIP_ROW_H));
            if enabled {
                strip_field(ui, session, player_id, version, appearance, cell);
            } else {
                gated_number(ui, tip, FIELD_W);
            }
        },
    );
    ui.add_space(CELL_GAP);
}

/// `cell`'s field in its `FIELD_W` slot.
fn strip_field(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
    cell: StripCell,
) {
    match cell {
        StripCell::BootsId => {
            id_box(
                ui,
                session,
                player_id,
                version,
                PlayerField::BootsId,
                appearance.boots_id,
                |p, v| {
                    p.appearance.boots_id = v;
                },
            );
        }
        StripCell::GlovesId => {
            id_box(
                ui,
                session,
                player_id,
                version,
                PlayerField::GlovesId,
                appearance.gloves_id,
                |p, v| {
                    p.appearance.gloves_id = v;
                },
            )
            .on_hover_text(
                "Any player can wear gloves, not only goalkeepers; 0 means normal hands.",
            );
        }
        StripCell::Enum(&(label, key, get, set)) => {
            combo_box(
                ui,
                session,
                player_id,
                label,
                &strip_options(version, key),
                get(appearance),
                move |player, shown| {
                    set(&mut player.appearance, shown);
                },
            );
        }
        StripCell::WristTaping => combo_box(
            ui,
            session,
            player_id,
            WRIST_TAPING_LABEL,
            &strip_options(version, SettingKey::WristTaping),
            appearance.wrist_taping,
            |player, shown| {
                player.appearance.wrist_taping = shown;
            },
        ),
        StripCell::WristTapeLeft => combo_box(
            ui,
            session,
            player_id,
            WRIST_TAPE_LEFT_LABEL,
            &strip_options(version, SettingKey::WristTapeColorLeft),
            appearance.wrist_tape_color_left,
            |player, shown| {
                player.appearance.wrist_tape_color_left = shown;
            },
        ),
        StripCell::WristTapeRight => combo_box(
            ui,
            session,
            player_id,
            WRIST_TAPE_RIGHT_LABEL,
            &strip_options(version, SettingKey::WristTapeColorRight),
            appearance.wrist_tape_color_right,
            |player, shown| {
                player.appearance.wrist_tape_color_right = shown;
            },
        ),
        StripCell::OutfielderGloves => {
            // PES 2014 stores the bit in the appearance record, the other
            // versions in the in-game face run.
            let pes14 = version == PesVersion::Pes14;
            let held = if pes14 {
                u8::from(appearance.player_gloves)
            } else {
                u8::from(face(appearance, IngameFaceField::PlayerGloves) != 0)
            };
            combo_box(
                ui,
                session,
                player_id,
                OUTFIELDER_GLOVES_LABEL,
                &gloves_options(version).map(str::to_owned),
                held,
                |player, shown| {
                    if pes14 {
                        player.appearance.player_gloves = shown != 0;
                    } else {
                        player
                            .appearance
                            .ingame_face
                            .set(IngameFaceField::PlayerGloves, shown)
                            .expect("a loaded player's face run holds player gloves");
                    }
                },
            );
        }
        StripCell::GlovesColor => {
            let stored = face(appearance, IngameFaceField::PlayerGlovesColor);
            combo_box(
                ui,
                session,
                player_id,
                GLOVES_COLOR_LABEL,
                &strip_options(version, SettingKey::GlovesColor),
                stored,
                |player, shown| {
                    player
                        .appearance
                        .ingame_face
                        .set(IngameFaceField::PlayerGlovesColor, shown)
                        .expect("a loaded player's face run holds the gloves colour");
                },
            );
        }
        StripCell::AnkleTaping => {
            checkbox(
                ui,
                session,
                player_id,
                "",
                appearance.ankle_taping,
                |p, v| {
                    p.appearance.ankle_taping = v;
                },
            );
        }
        StripCell::Spectacles => combo_box(
            ui,
            session,
            player_id,
            SPECTACLES_LABEL,
            &strip_options(version, SettingKey::Spectacles),
            appearance.spectacles_style,
            |player, shown| {
                player.appearance.spectacles_style = shown;
            },
        ),
        StripCell::SpectaclesColor => {
            combo_box(
                ui,
                session,
                player_id,
                SPECTACLES_COLOR_LABEL,
                &strip_options(version, SettingKey::SpectaclesColor),
                appearance.spectacles_color,
                |player, shown| {
                    player.appearance.spectacles_color = shown;
                },
            );
        }
        StripCell::ShirttailOut => {
            checkbox(ui, session, player_id, "", appearance.untucked, |p, v| {
                p.appearance.untucked = v;
            });
        }
    }
}

/// A strip cell's ID box: a `FIELD_W` `DragValue` over the record's own
/// bound (`player_max_value`, wider than `settings.toml`'s stock range —
/// a compile-allocated ID such as 627 is a value the save holds), the
/// wheel and arrow-key steps `abilities`' `number` gives.
fn id_box(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    field: PlayerField,
    stored: u32,
    set: impl FnOnce(&mut PlayerEntry, u32),
) -> egui::Response {
    let range = 0..=u32::try_from(
        player_max_value(version, field).expect("the id is stored on every version"),
    )
    .expect("the id's bound fits u32");
    let mut shown = stored;
    let height = ui.spacing().interact_size.y;
    let mut response = number_arrows(ui, &mut shown, range.clone(), |ui, value| {
        ui.add(number_box(
            egui::DragValue::new(value)
                .range(range.clone())
                .clamp_existing_to_range(false)
                .min_size(egui::vec2(FIELD_W, height)),
        ))
    });
    if step_number(&mut shown, number_wheel_steps(ui, &response), range) {
        response.mark_changed();
    }
    if response.changed() {
        edit_player(session, player_id, response.id.value(), |player| {
            set(player, shown);
        });
    }
    response
}

/// The Motion group: `motions(version)`'s nine controls in the draft's
/// order as label/field pairs, two a row, the dribbling motion between
/// the kicks and the celebrations (PES 20+ and PES 2014, a version without
/// it drawing it disabled with the version tooltip), Randomize under the
/// grid.
fn motion_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    motion: &PlayerMotion,
    draw: &mut dyn FnMut(u8) -> u8,
    schema: &VersionSchema,
) {
    // Two pairs a row (the draft's `COL_PANELS` motion `[2, 194, 14]`).
    // The boxes at the draft's number-input width; no column floor, so
    // the label columns keep their labels' width.
    let fields = motions(version);
    let per_row = 2;
    egui::Grid::new("appearance motion")
        .min_col_width(0.0)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FIELD_SPACING))
        .show(ui, |ui| {
            ui.spacing_mut().interact_size.x = NUMBER_BOX;
            let mut pair = 0usize;
            for field in &fields[..7] {
                motion_number(ui, session, player_id, version, field, motion, schema);
                pair += 1;
                maybe_end_row(ui, pair, per_row);
            }
            // The dribbling motion sits between the kicks and the celebrations
            // (the draft's fourth row's last cell); a version without it draws
            // it disabled at 0 with the `PES NN+` tooltip.
            if motion.dribbling.is_some() {
                ui.label(DRIBBLING_MOTION_LABEL);
            } else {
                gated_label(
                    ui,
                    DRIBBLING_MOTION_LABEL,
                    gating_tooltip(PlayerField::DribblingMotion),
                );
            }
            match motion.dribbling {
                Some(stored) => {
                    // Shown from 1, as the motions beside it are.
                    let mut shown = stored + 1;
                    let range = dribbling_range();
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
                            player.motion.dribbling = Some(shown - 1);
                        });
                    }
                }
                None => {
                    add_enabled_ui(ui, false, |ui| {
                        ui.add(number_box(
                            egui::DragValue::new(&mut 1u8).range(dribbling_range()),
                        ));
                    })
                    .response
                    .on_disabled_hover_text(gating_tooltip(PlayerField::DribblingMotion));
                }
            }
            pair += 1;
            maybe_end_row(ui, pair, per_row);
            for field in &fields[7..] {
                motion_number(ui, session, player_id, version, field, motion, schema);
                pair += 1;
                maybe_end_row(ui, pair, per_row);
            }
        });
    // Every motion but the dribbling motion, from 1 to the version's shown
    // top, one undoable edit. The measured tops, not two fixed sets, so a
    // draw never writes a motion the game does not have and reaches every
    // one it has.
    if ui
        .button("Randomize")
        .on_hover_text("Set every motion to a random value, except the dribbling motion")
        .clicked()
    {
        randomize(session, player_id, version, draw);
    }
}

/// One motion's label/`DragValue` pair on the shown scale: a 1-based motion
/// shows 1 to `motion_top` + 1 (stored one below), a celebration 0 to the
/// top (stored as shown). A motion the version does not store draws the
/// gated stand-in.
fn motion_number(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    field: &MotionField,
    motion: &PlayerMotion,
    schema: &VersionSchema,
) {
    if !schema.player_has(field.field) {
        gated_label(ui, field.label, gating_tooltip(field.field));
        gated_number(ui, gating_tooltip(field.field), NUMBER_BOX);
        return;
    }
    let top = motion_top(version, field.field).expect("a stored motion has a measured top");
    let held = (field.get)(motion);
    let (shown, range) = if field.one_based {
        (held + 1, 1..=top + 1)
    } else {
        (held, 0..=top)
    };
    let set = field.set;
    let one_based = field.one_based;
    number(
        ui,
        session,
        player_id,
        field.label,
        shown,
        range,
        move |player, shown| {
            set(
                &mut player.motion,
                if one_based { shown - 1 } else { shown },
            );
        },
    );
}

/// The dribbling motion's shown range, 1 to its key's top: the
/// `settings.toml` key's, whose stored values count from 0.
fn dribbling_range() -> std::ops::RangeInclusive<u8> {
    let Kind::OneBased { max } = SettingKey::Dribbling.spec().kind else {
        unreachable!("the dribbling motion's key counts from 1")
    };
    1..=max
}

/// Randomize (the draft's button): every motion but the dribbling motion
/// drawn from 1 to its shown top, the writes landing as one undo entry.
fn randomize(
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    draw: &mut dyn FnMut(u8) -> u8,
) {
    edit_player(
        session,
        player_id,
        egui::Id::unique(("appearance randomize", player_id)).value(),
        |player| {
            for field in motions(version) {
                // A motion the version does not store has no top and is
                // left alone (its stand-in is drawn disabled).
                let Some(top) = motion_top(version, field.field) else {
                    continue;
                };
                let shown_top = if field.one_based { top + 1 } else { top };
                let drawn = draw(shown_top);
                (field.set)(
                    &mut player.motion,
                    if field.one_based { drawn - 1 } else { drawn },
                );
            }
        },
    );
}

/// The Colors group's two swatch rows (the draft's Colours group): skin
/// and iris, each its palette's entries as swatches, the two rows' first
/// squares at one x. The skin palette drops the Custom entry on Fox (PES
/// 15 to 17 alone have it, the three pre-Fox versions).
fn colour_rows(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
    face_run: bool,
) {
    egui::Grid::new("appearance colours")
        .num_columns(2)
        .min_col_width(0.0)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, SWATCH_ROW_SPACING))
        .show(ui, |ui| {
            for key in [SettingKey::SkinColor, SettingKey::IrisColor] {
                let field = key
                    .face_field()
                    .expect("a colour palette's is a face field");
                let palette = palette(version, key);
                // A version without an in-game face run draws the rows
                // greyed: no stored value to read and nothing to write.
                let stored = if face_run { face(appearance, field) } else { 0 };
                if face_run {
                    ui.label(palette.label);
                } else {
                    gated_label(ui, palette.label, PES14_TIP.to_owned());
                }
                // A scope, not a horizontal: a horizontal claims the whole
                // remaining width as its frame, and the grid's last column
                // would carry it to the pane's edge. `add_space` adds to
                // the item spacing already claimed after the swatches.
                ui.scope(|ui| {
                    add_enabled_ui(ui, face_run, |ui| {
                        palette_row(ui, session, player_id, &palette, stored, |player, shown| {
                            player.appearance.ingame_face.set(field, shown).expect(
                                "a loaded player's face run holds the section's colour fields",
                            );
                        });
                    })
                    .response
                    .on_disabled_hover_text(PES14_TIP);
                    ui.add_space(SWATCH_CELL_PAD - ui.spacing().item_spacing.x);
                });
                ui.end_row();
            }
        });
}

/// One checkbox pair, the label inside the widget (the edit flags'
/// shape; `""` draws the bare box — the strip grid's checkboxes'). A
/// change writes `set` through `edit_player` under the widget's id.
fn checkbox(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    label: &'static str,
    held: bool,
    set: impl FnOnce(&mut PlayerEntry, bool),
) {
    let mut value = held;
    let response = studio_core::widgets::checkbox(ui, &mut value, label);
    if response.changed() {
        edit_player(session, player_id, response.id.value(), |player| {
            set(player, value);
        });
    }
}

/// A strip cell's `ComboBox`, `FIELD_W` wide, `options` its rows' labels
/// in pick order (`combo_options`' for a `SettingKey`, `GLOVES_OPTIONS`' for
/// the outfielder gloves); `stored` the held index, its number shown when
/// past the list. A pick, or a wheel notch stepping through the rows,
/// writes `set`.
fn combo_box(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    label: &'static str,
    options: &[String],
    stored: u8,
    set: impl FnOnce(&mut PlayerEntry, u8),
) {
    let mut picked = stored;
    let selected = options
        .get(usize::from(stored))
        .cloned()
        .unwrap_or_else(|| stored.to_string());
    let index = |at: usize| u8::try_from(at).expect("the options fit a u8");
    let combo = egui::ComboBox::from_id_salt(("appearance", label))
        .selected_text(selected)
        .width(FIELD_W)
        .show_ui(ui, |ui| {
            for (at, option) in options.iter().enumerate() {
                combo_item(ui, &mut picked, index(at), option);
            }
        });
    let steps = wheel_steps(ui, &combo.response);
    if let Some(next) = step_choice((0..options.len()).map(index), &picked, steps) {
        picked = next;
    }
    if picked != stored {
        edit_player(session, player_id, combo.response.id.value(), |player| {
            set(player, picked)
        });
    }
}

/// One palette row (the draft's `paletteRow`): a framed swatch per palette
/// entry 0 to `palette.top`, the entry's name or index in its widget label
/// and hover; a stored value past `top` draws its number after the
/// swatches. A click writes the swatch's index through `edit_player`.
fn palette_row(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    palette: &Palette,
    stored: u8,
    set: impl Fn(&mut PlayerEntry, u8),
) {
    // The swatch names once per row (each is `option_name`'s entry: the
    // palette keys' `Kind::Number` ranges start at 0, so `combo_options`
    // holds one entry per index up to the key's top, and every drawn
    // index has one).
    let names = combo_options(palette.key);
    ui.push_id(("appearance palette", palette.label), |ui| {
        let item_gap = ui.spacing().item_spacing.x;
        ui.spacing_mut().item_spacing.x = SWATCH_GAP;
        for (index, rgb) in palette.colors[..=usize::from(palette.top)]
            .iter()
            .enumerate()
        {
            let name = &names[index];
            let widget_label = format!("{} {name}", palette.label);
            let response =
                swatch(ui, *rgb, index == usize::from(stored), &widget_label).on_hover_text(name);
            if response.clicked() {
                edit_player(session, player_id, response.id.value(), |player| {
                    set(
                        player,
                        u8::try_from(index).expect("a palette has at most eleven entries"),
                    );
                });
            }
        }
        if stored > palette.top {
            ui.add_space(item_gap - SWATCH_GAP);
            ui.weak(stored.to_string());
        }
    });
}

/// One palette entry's framed square (the draft's `.swatch`): the colour in
/// a dark frame, the held entry outlined as `.on` is.
fn swatch(ui: &mut egui::Ui, rgb: [u8; 3], held: bool, label: &str) -> egui::Response {
    let size = egui::vec2(14.0, 14.0);
    let (rect, response) = ui.allocate_at_least(size, egui::Sense::click());
    response.widget_info(|| {
        let mut info =
            egui::WidgetInfo::labeled(egui::Role::ColorWell, ui.is_enabled(), label.to_owned());
        info.selected = Some(held);
        info
    });
    if ui.is_rect_visible(rect) {
        let radius = ui.visuals().widgets.noninteractive.corner_radius;
        ui.painter().rect_filled(
            rect,
            radius,
            egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        );
        ui.painter().rect_stroke(
            rect,
            radius,
            egui::Stroke::new(1.0, studio_core::theme::palette(ui.visuals()).swatch_edge),
            egui::StrokeKind::Inside,
        );
        if held {
            ui.painter().rect_stroke(
                rect.expand(1.0),
                radius,
                egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Outside,
            );
        }
    }
    response
}

/// `field`'s value inside `appearance`'s ingame-face run: the run a loaded
/// save's players always carry, and a field `layout()` names for every
/// version (the four the section edits are `None`, both layouts).
fn face(appearance: &PlayerAppearance, field: IngameFaceField) -> u8 {
    appearance
        .ingame_face
        .get(field)
        .expect("a loaded player's face run holds the section's colour fields")
}

/// A xorshift64 (13, 7, 17) draw in `1..=top` from `seed` (the Randomize
/// source). The picks need only be unpredictable to a member, not
/// cryptographic.
pub(crate) fn xorshift_draw(seed: u64) -> impl FnMut(u8) -> u8 {
    let mut state = seed;
    move |top| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        1 + u8::try_from(state % u64::from(top.max(1))).expect("a remainder below a u8 top fits u8")
    }
}

/// `xorshift_draw` seeded from the wall clock (`SaveView::new` wires it as
/// `randomize_draw`; the tests substitute a fixed source).
pub(crate) fn seeded_draw() -> impl FnMut(u8) -> u8 {
    xorshift_draw(clock_seed())
}

/// The wall-clock seed for `seeded_draw`, forced odd: a zero state draws 1
/// forever.
fn clock_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| odd_seed(u64::from(since.subsec_nanos()), since.as_secs()))
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

/// `clock_seed`'s arithmetic as a pure function (its test needs even
/// inputs, which a real clock does not hand it on demand): `nanos ^ secs`
/// forced odd, a zero state drawing 1 forever.
fn odd_seed(nanos: u64, secs: u64) -> u64 {
    nanos ^ secs | 1
}

#[cfg(test)]
mod tests {
    use super::{clock_seed, odd_seed, seeded_draw, xorshift_draw};

    // The Randomize generator is xorshift64 (13, 7, 17): each draw is
    // `1 + state % top` after one round of the three shifts. The first
    // state from seed 1 is 1082269761; the three draws' states are stepped
    // out here by hand, so the test never calls the code under test.
    #[test]
    fn xorshift_draw_returns_the_states_remainders_plus_one() {
        let mut state = 1u64;
        let mut expected = Vec::new();
        for _ in 0..3 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            expected.push(1 + u8::try_from(state % 162).expect("a remainder fits u8"));
        }
        let mut first_state = 1u64;
        first_state ^= first_state << 13;
        first_state ^= first_state >> 7;
        first_state ^= first_state << 17;
        assert_eq!(first_state, 1082269761, "the pinned first state");
        let mut draw = xorshift_draw(1);
        let drawn: Vec<u8> = (0..3).map(|_| draw(162)).collect();
        assert_eq!(drawn, expected);
    }

    // The clock seed must be odd (a zero state draws 1 forever).
    #[test]
    fn the_clock_seed_is_odd() {
        assert_eq!(clock_seed() % 2, 1, "an even or zero seed draws 1 forever");
    }

    // The forcing is the `| 1`: on even inputs `& 1` would hand back 0.
    #[test]
    fn the_odd_seed_is_odd_on_even_inputs() {
        // Exact values: an odd-only check passes a constant 1, and on
        // (2, 4) `| 1` -> `^ 1` gives 7 too; on (1, 2) it gives 2.
        assert_eq!(odd_seed(2, 4), 7, "2 ^ 4 forced odd");
        assert_eq!(odd_seed(1, 2), 3, "1 ^ 2, already odd, kept");
    }

    // `seeded_draw` returns values in `1..=top`, both ends occurring over a
    // long enough run (the source need only be unpredictable to a member).
    #[test]
    fn the_seeded_draw_covers_the_whole_range() {
        let mut draw = seeded_draw();
        for top in [1u8, 5, 162] {
            let (mut low, mut high) = (u8::MAX, u8::MIN);
            for _ in 0..10_000 {
                let value = draw(top);
                assert!((1..=top).contains(&value), "{value} out of 1..={top}");
                low = low.min(value);
                high = high.max(value);
            }
            assert_eq!((low, high), (1, top), "top {top}: both ends occur");
        }
    }
}
